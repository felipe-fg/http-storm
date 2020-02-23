use super::metric::RequestMetric;
use super::settings::Settings;
use super::worker::WorkerMessage;
use std::cmp;
use std::io::{stdout, Read};
use std::time;
use termion::async_stdin;
use termion::raw::IntoRawMode;
use tokio;
use tokio::sync::mpsc::UnboundedReceiver;
use tui::backend::{Backend, TermionBackend};
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Text, Widget};
use tui::{Frame, Terminal};

pub async fn render(settings: &Settings, mut receiver: UnboundedReceiver<WorkerMessage>) -> () {
    let mut stdin = async_stdin().bytes();

    let mut terminal = create_terminal();

    let mut metrics = Vec::new();

    'render: loop {
        while let Some(Ok(input)) = stdin.next() {
            if input == b'q' {
                break 'render;
            }
        }

        while let Ok(message) = receiver.try_recv() {
            metrics.push(message.metric);
        }

        terminal
            .draw(|frame| {
                draw(settings, &metrics, frame);
            })
            .expect("terminal draw");

        tokio::time::delay_for(time::Duration::from_millis(100)).await;
    }
}

fn create_terminal() -> Terminal<impl Backend> {
    let stdout = stdout().into_raw_mode().expect("stdout");
    let backend = TermionBackend::new(stdout);

    let mut terminal = Terminal::new(backend).expect("terminal create");
    terminal.clear().expect("terminal clear");
    terminal.autoresize().expect("terminal autoresize");
    terminal.hide_cursor().expect("terminal cursor");

    terminal
}

fn draw(settings: &Settings, metrics: &Vec<RequestMetric>, mut f: Frame<impl Backend>) -> () {
    let size = f.size();

    let progress = settings
        .total
        .map(|total| (metrics.len() as f64 / total as f64) * 100f64)
        .map(|percent| cmp::min(percent.ceil() as u16, 100))
        .unwrap_or(0u16);

    let durations = metrics
        .iter()
        .skip((cmp::max(metrics.len() as i64 - size.width as i64, 0)) as usize)
        .take(size.width as usize)
        .map(|metric| metric.elapsed_time.num_milliseconds() as u64)
        .collect::<Vec<u64>>();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(10),
                Constraint::Percentage(5),
                Constraint::Percentage(20),
                Constraint::Percentage(65),
            ]
            .as_ref(),
        )
        .split(f.size());

    Paragraph::new([Text::raw("HTTP Storm")].iter())
        .style(Style::default().fg(Color::Blue).modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .render(&mut f, chunks[0]);

    Gauge::default()
        .style(Style::default().bg(Color::DarkGray).fg(Color::Gray))
        .label("")
        .percent(progress)
        .render(&mut f, chunks[1]);

    Sparkline::default()
        .style(Style::default().fg(Color::LightGreen))
        .data(&durations)
        .render(&mut f, chunks[2]);
}
