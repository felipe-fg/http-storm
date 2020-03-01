use crate::settings::Settings;
use crate::summary;
use crate::worker::{WorkerCommand, WorkerMessage};
use std::io::{self, stdout, Bytes, Read};
use std::time;
use termion::async_stdin;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::AsyncReader;
use tokio::{self, sync::mpsc, sync::watch};
use tui::backend::{Backend, TermionBackend};
use tui::{Frame, Terminal};

#[derive(Debug)]
enum InputCommand {
    Summary,
    Quit,
    None,
}

impl InputCommand {
    fn from_input(input: io::Result<u8>) -> InputCommand {
        if let Ok(key) = input {
            match key {
                b'q' | b'Q' => InputCommand::Quit,
                b's' | b'S' => InputCommand::Summary,
                _ => InputCommand::None,
            }
        } else {
            InputCommand::None
        }
    }
}

pub async fn render(
    settings: &Settings,
    mut receiver: mpsc::UnboundedReceiver<WorkerMessage>,
    sender: watch::Sender<WorkerCommand>,
) -> () {
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

        'input: while let Some(input) = terminal_stdin.next() {
            match InputCommand::from_input(input) {
                InputCommand::Summary => {
                    match sender.broadcast(WorkerCommand::Stop) {
                        _ => (),
                    };

                    receiver.close();

                    break 'input;
                }
                InputCommand::Quit => {
                    break 'render;
                }
                _ => (),
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

    let summary = summary::compute(&messages, settings, columns);

    view::draw(&summary, frame);
}

mod view {
    use crate::summary::Summary;
    use tui::backend::Backend;
    use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use tui::style::{Color, Modifier, Style};
    use tui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Text, Widget};
    use tui::Frame;

    pub fn draw(summary: &Summary, mut frame: Frame<impl Backend>) -> () {
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
        draw_progress(&summary, &mut frame, chunks[1]);
        draw_timeline(&summary, &mut frame, chunks[2]);
        draw_request(&summary, &mut frame, chunks[3]);
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

    fn draw_progress(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Gauge::default()
            .block(default_block())
            .style(default_style(Color::Gray).bg(Color::DarkGray))
            .percent(summary.progress_percent)
            .render(frame, chunk);
    }

    fn draw_timeline(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        Sparkline::default()
            .block(default_block())
            .style(default_style(Color::LightGreen))
            .data(&summary.stats.time_values)
            .max(summary.stats.time_mean * 2)
            .render(frame, chunk);
    }

    fn draw_request(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
        let method = Text::styled(&summary.request_method, bold_style(Color::Green));
        let url = Text::styled(&summary.request_url, bold_style(Color::Blue));

        let count = Text::styled(
            format!("{} requests", &summary.total_count),
            bold_style(Color::Gray),
        );

        let elapsed = Text::styled(
            format!("{}s", &summary.elapsed_seconds),
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
