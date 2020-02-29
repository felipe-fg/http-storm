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
    if let Ok(key) = input {
        key == b'q' || key == b'Q'
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
        pub timeline_min: u64,
        pub timeline_max: u64,
        pub timeline_avg: u64,

        pub request_method: String,
        pub request_url: String,
        pub request_count: u64,
        pub request_total: u64,
        pub request_elapsed: u64,
        pub request_duration: u64,
    }

    pub fn build(messages: &Vec<WorkerMessage>, settings: &Settings, columns: usize) -> State {
        let elapsed_time = build_elapsed_time(messages);
        let progress_percent = build_progress_percent(messages, settings, elapsed_time);

        let timeline_durations = build_timeline_durations(messages, columns);
        let timeline_min = build_timeline_min(&timeline_durations);
        let timeline_max = build_timeline_max(&timeline_durations);
        let timeline_avg = build_timeline_avg(&timeline_durations);

        let request_method = settings.method.to_string();
        let request_url = settings.url.to_string();
        let request_count = messages.len() as u64;
        let request_total = settings.total.unwrap_or(0) as u64;
        let request_elapsed = elapsed_time.num_seconds() as u64;
        let request_duration = settings.duration.unwrap_or(0) as u64;

        State {
            elapsed_time: elapsed_time,
            progress_percent: progress_percent,

            timeline_durations: timeline_durations,
            timeline_min: timeline_min,
            timeline_max: timeline_max,
            timeline_avg: timeline_avg,

            request_method: request_method,
            request_url: request_url,
            request_count: request_count,
            request_total: request_total,
            request_elapsed: request_elapsed,
            request_duration: request_duration,
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

    fn build_timeline_min(durations: &Vec<u64>) -> u64 {
        let min = durations.iter().min_by(|x, y| x.cmp(y));

        if let Some(min) = min {
            *min
        } else {
            0
        }
    }

    fn build_timeline_max(durations: &Vec<u64>) -> u64 {
        let max = durations.iter().max_by(|x, y| x.cmp(y));

        if let Some(max) = max {
            *max
        } else {
            0
        }
    }

    fn build_timeline_avg(durations: &Vec<u64>) -> u64 {
        let sum = durations.iter().sum::<u64>();
        let count = durations.len();

        if !durations.is_empty() {
            let average = sum as f64 / count as f64;

            average.ceil() as u64
        } else {
            0
        }
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
                    Constraint::Min(6),
                    Constraint::Length(5),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(frame.size());

        draw_header(&mut frame, chunks[0]);
        draw_progress(&state, &mut frame, chunks[1]);
        draw_timeline(&state, &mut frame, chunks[2]);
        draw_request(&state, &mut frame, chunks[3]);
        draw_footer(&mut frame, chunks[4]);
    }

    fn draw_header(frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        let text = [Text::raw("HTTP Storm")];

        Paragraph::new(text.iter())
            .block(default_block())
            .style(bold_style(Color::Blue))
            .alignment(Alignment::Center)
            .render(frame, chunk);
    }

    fn draw_progress(state: &State, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Gauge::default()
            .block(default_block())
            .style(default_style(Color::Gray).bg(Color::DarkGray))
            .percent(state.progress_percent)
            .render(frame, chunk);
    }

    fn draw_timeline(state: &State, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Sparkline::default()
            .block(default_block())
            .style(default_style(Color::LightGreen))
            .data(&state.timeline_durations)
            .max(state.timeline_avg * 2)
            .render(frame, chunk);
    }

    fn draw_request(state: &State, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        let method = Text::styled(&state.request_method, bold_style(Color::Green));
        let url = Text::styled(&state.request_url, bold_style(Color::Blue));

        let count = Text::styled(
            format!("{} requests", &state.request_count),
            bold_style(Color::Gray),
        );

        let elapsed = Text::styled(
            format!("{}s", &state.request_elapsed),
            bold_style(Color::Gray),
        );

        let text = [
            method,
            Text::raw(" "),
            url,
            Text::raw("\n"),
            count,
            Text::raw("\n"),
            elapsed,
        ];

        Paragraph::new(text.iter())
            .block(default_block())
            .style(default_style(Color::Gray))
            .alignment(Alignment::Left)
            .render(frame, chunk);
    }

    fn draw_footer(frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunk);

        let quit_text = [
            Text::raw("("),
            Text::styled("S", bold_style(Color::Gray)),
            Text::raw(")ummary"),
            Text::raw(" / ("),
            Text::styled("Q", bold_style(Color::Gray)),
            Text::raw(")uit"),
        ];

        Paragraph::new(quit_text.iter())
            .block(default_block())
            .style(default_style(Color::Gray))
            .alignment(Alignment::Left)
            .render(frame, chunks[0]);

        Paragraph::new([Text::raw("http-storm/0.1.0")].iter())
            .block(default_block())
            .style(default_style(Color::DarkGray))
            .alignment(Alignment::Right)
            .render(frame, chunks[1]);
    }

    fn default_block<'a>() -> Block<'a> {
        Block::default()
            .border_style(default_style(Color::Black))
            .borders(Borders::ALL)
    }

    fn default_style(fg: Color) -> Style {
        Style::default().bg(Color::Black).fg(fg)
    }

    fn bold_style(fg: Color) -> Style {
        default_style(fg).modifier(Modifier::BOLD)
    }
}
