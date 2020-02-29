use crate::settings::Settings;
use crate::worker::WorkerMessage;
use std::io::{self, stdout, Bytes, Read};
use std::time;
use termion::async_stdin;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::AsyncReader;
use tokio::{self, sync::mpsc::UnboundedReceiver};
use tui::backend::{Backend, TermionBackend};
use tui::{Frame, Terminal};

pub async fn render(settings: &Settings, mut receiver: UnboundedReceiver<WorkerMessage>) -> () {
    let mut terminal = create_terminal().expect("terminal");
    let mut terminal_stdin = create_terminal_stdin();

    let mut messages = Vec::new();

    'render: loop {
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }

        terminal
            .draw(|frame| update_terminal(&messages, settings, frame))
            .expect("draw");

        while let Some(input) = terminal_stdin.next() {
            if check_quit(input) {
                receiver.close();

                break 'render;
            }
        }

        tokio::time::delay_for(time::Duration::from_millis(250)).await;
    }
}

fn create_terminal() -> io::Result<Terminal<impl Backend>> {
    let raw_terminal = stdout().into_raw_mode()?;
    let alternate_screen = AlternateScreen::from(raw_terminal);
    let termion_backend = TermionBackend::new(alternate_screen);

    let mut terminal = Terminal::new(termion_backend)?;
    terminal.clear()?;
    terminal.autoresize()?;
    terminal.hide_cursor()?;

    Ok(terminal)
}

fn create_terminal_stdin() -> Bytes<AsyncReader> {
    async_stdin().bytes()
}

fn update_terminal(
    messages: &Vec<WorkerMessage>,
    settings: &Settings,
    frame: Frame<impl Backend>,
) -> () {
    let border = 2;
    let size = frame.size();
    let columns = (size.width - border) as usize;

    let state = state::build(&messages, settings, columns);

    view::draw(&state, frame);
}

fn check_quit(input: io::Result<u8>) -> bool {
    if let Ok(b'q') = input {
        true
    } else {
        false
    }
}

mod state {
    use crate::settings::Settings;
    use crate::worker::WorkerMessage;
    use chrono::Duration;
    use std::cmp;

    #[derive(Debug)]
    pub struct State {
        pub elapsed_time: Duration,
        pub progress_percent: u16,
        pub timeline_durations: Vec<u64>,
        pub timeline_max: u64,
    }

    pub fn build(messages: &Vec<WorkerMessage>, settings: &Settings, columns: usize) -> State {
        let elapsed_time = build_elapsed_time(messages);
        let progress_percent = build_progress_percent(messages, settings, elapsed_time);
        let timeline_durations = build_timeline_durations(messages, columns);
        let timeline_max = build_timeline_max(&timeline_durations);

        State {
            elapsed_time: elapsed_time,
            progress_percent: progress_percent,
            timeline_durations: timeline_durations,
            timeline_max: timeline_max,
        }
    }

    fn build_elapsed_time(messages: &Vec<WorkerMessage>) -> Duration {
        let id = 1;
        let message = messages.iter().rev().find(|message| message.id == id);

        if let Some(message) = message {
            message.elapsed_time
        } else {
            Duration::zero()
        }
    }

    fn build_progress_percent(
        messages: &Vec<WorkerMessage>,
        settings: &Settings,
        elapsed: Duration,
    ) -> u16 {
        let total_progress = if let Some(total) = settings.total {
            let ratio = messages.len() as f64 / total as f64;

            cmp::min((ratio * 100f64).ceil() as u16, 100)
        } else {
            0u16
        };

        let duration_progress = if let Some(duration) = settings.duration {
            let ratio = elapsed.num_seconds() as f64 / duration as f64;

            cmp::min((ratio * 100f64).ceil() as u16, 100)
        } else {
            0u16
        };

        cmp::max(total_progress, duration_progress)
    }

    fn build_timeline_durations(messages: &Vec<WorkerMessage>, columns: usize) -> Vec<u64> {
        let skip = cmp::max(messages.len() as i64 - columns as i64, 0) as usize;

        let page = messages.iter().skip(skip).take(columns);
        let durations = page.map(|message| message.metric.elapsed_time.num_milliseconds() as u64);

        durations.collect::<Vec<u64>>()
    }

    fn build_timeline_max(durations: &Vec<u64>) -> u64 {
        let sum = durations.iter().sum::<u64>();
        let count = durations.len();

        let average = sum as f64 / count as f64;

        (average * 2f64).ceil() as u64
    }
}

mod view {
    use crate::ui::state::State;
    use tui::backend::Backend;
    use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use tui::style::{Color, Modifier, Style};
    use tui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Text, Widget};
    use tui::Frame;

    pub fn draw(state: &State, mut frame: Frame<impl Backend>) -> () {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(6),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(frame.size());

        draw_header(&mut frame, chunks[0]);
        draw_progress(&state, &mut frame, chunks[1]);
        draw_timeline(&state, &mut frame, chunks[2]);
    }

    fn draw_header(frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Paragraph::new([Text::raw("HTTP Storm")].iter())
            .block(
                Block::default()
                    .border_style(Style::default().bg(Color::Black).fg(Color::Black))
                    .borders(Borders::ALL),
            )
            .style(
                Style::default()
                    .bg(Color::Black)
                    .fg(Color::Blue)
                    .modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .render(frame, chunk);
    }

    fn draw_progress(state: &State, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Gauge::default()
            .block(
                Block::default()
                    .border_style(Style::default().bg(Color::Black).fg(Color::Black))
                    .borders(Borders::ALL),
            )
            .style(Style::default().bg(Color::DarkGray).fg(Color::Gray))
            .percent(state.progress_percent)
            .render(frame, chunk);
    }

    fn draw_timeline(state: &State, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Sparkline::default()
            .block(
                Block::default()
                    .border_style(Style::default().bg(Color::Black).fg(Color::Black))
                    .borders(Borders::ALL),
            )
            .style(Style::default().bg(Color::Black).fg(Color::LightGreen))
            .data(&state.timeline_durations)
            .max(state.timeline_max)
            .render(frame, chunk);
    }
}
