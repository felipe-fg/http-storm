use crate::summary::Summary;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Text, Widget};
use tui::Frame;

pub fn draw_running(summary: &Summary, mut frame: Frame<impl Backend>) -> () {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(frame.size());

    let widgets = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(layout[1]);

    let title = "HTTP Storm";
    let commands = vec!["Stop", "Quit"];
    let version = "http-storm/0.1.0";

    draw_layout_header(title, &mut frame, layout[0]);
    draw_widget_progress(&summary, &mut frame, widgets[0]);
    draw_widget_timeline(&summary, &mut frame, widgets[1]);
    draw_widget_request(&summary, &mut frame, widgets[2]);
    draw_layout_footer(&commands, version, &mut frame, layout[2]);
}

pub fn draw_finished(summary: &Summary, mut frame: Frame<impl Backend>) -> () {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(frame.size());

    let widgets = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(20)].as_ref())
        .split(layout[1]);

    let title = "HTTP Storm";
    let commands = vec!["Quit"];
    let version = "http-storm/0.1.0";

    draw_layout_header(title, &mut frame, layout[0]);
    draw_widget_request(&summary, &mut frame, widgets[0]);
    draw_widget_stats(&summary, &mut frame, widgets[1]);
    draw_layout_footer(&commands, version, &mut frame, layout[2]);
}

fn draw_layout_header(title: &str, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
    Paragraph::new([Text::raw(title)].iter())
        .block(block_default())
        .style(style_bold(Color::Blue))
        .alignment(Alignment::Center)
        .render(frame, chunk);
}

fn draw_layout_footer(
    shortcuts: &[&str],
    version: &str,
    frame: &mut Frame<impl Backend>,
    chunk: Rect,
) -> () {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunk);

    let mut text_shortcuts = Vec::new();

    for shortcut in shortcuts {
        let head = &shortcut[0..1];
        let tail = &shortcut[1..];

        text_shortcuts.push(Text::raw("("));
        text_shortcuts.push(Text::styled(head.to_string(), style_bold(Color::Gray)));
        text_shortcuts.push(Text::raw(")"));
        text_shortcuts.push(Text::raw(tail.to_string()));

        if let Some(last) = shortcuts.last() {
            if shortcut != last {
                text_shortcuts.push(Text::raw(" / "));
            }
        }
    }

    Paragraph::new(text_shortcuts.iter())
        .block(block_default())
        .style(style_default(Color::Gray))
        .alignment(Alignment::Left)
        .render(frame, chunks[0]);

    Paragraph::new([Text::raw(version)].iter())
        .block(block_default())
        .style(style_default(Color::DarkGray))
        .alignment(Alignment::Right)
        .render(frame, chunks[1]);
}

fn draw_widget_progress(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
    Gauge::default()
        .block(block_default())
        .style(style_default(Color::Gray).bg(Color::DarkGray))
        .percent(summary.progress_percent)
        .render(frame, chunk);
}

fn draw_widget_timeline(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
    Sparkline::default()
        .block(block_default())
        .style(style_default(Color::LightGreen))
        .data(&summary.stats.time_values)
        .max(summary.stats.time_mean * 2)
        .render(frame, chunk);
}

fn draw_widget_request(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
    let method = Text::styled(&summary.request_method, style_bold(Color::Green));
    let url = Text::styled(&summary.request_url, style_bold(Color::Blue));

    let count = Text::styled(
        format!("{} requests", &summary.total_count),
        style_bold(Color::Gray),
    );

    let elapsed = Text::styled(
        format!("{}s", &summary.elapsed_seconds),
        style_bold(Color::Gray),
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
        .block(block_default())
        .style(style_default(Color::Gray))
        .alignment(Alignment::Left)
        .render(frame, chunk);
}

fn draw_widget_stats(summary: &Summary, frame: &mut Frame<impl Backend>, chunk: Rect) -> () {
    let mut text = Vec::new();

    let rate = summary.total_count as f64 / summary.elapsed_seconds as f64;

    text.push(Text::styled(
        format!("Rate: {:.2}req/s\n", rate),
        style_bold(Color::Gray),
    ));

    text.push(Text::styled(
        format!("Fastest: {}ms\n", summary.stats.time_minimum),
        style_bold(Color::Gray),
    ));

    text.push(Text::styled(
        format!("Slowest: {}ms\n", summary.stats.time_maximum),
        style_bold(Color::Gray),
    ));

    text.push(Text::styled(
        format!("Mean: {}ms\n", summary.stats.time_mean),
        style_bold(Color::Gray),
    ));

    text.push(Text::styled(
        format!("Standard Deviation: {}ms\n", summary.stats.time_stddev),
        style_bold(Color::Gray),
    ));

    text.push(Text::styled(format!("\n"), style_bold(Color::Gray)));

    for ((lower, upper), count) in &summary.stats.time_histogram {
        text.push(Text::styled(
            format!("{}ms ~ {}ms: {}\n", lower, upper, count),
            style_bold(Color::Gray),
        ));
    }

    text.push(Text::styled(format!("\n"), style_bold(Color::Gray)));

    for (status, count) in &summary.stats.status {
        let color = match &status[0..1] {
            "2" => Color::Green,
            "3" => Color::Yellow,
            "4" => Color::Red,
            "5" => Color::Red,
            "F" => Color::Red,
            _ => Color::Gray,
        };

        text.push(Text::styled(
            format!("Status {}: {}\n", status, count),
            style_bold(color),
        ));
    }

    Paragraph::new(text.iter())
        .block(block_default())
        .style(style_default(Color::Gray))
        .alignment(Alignment::Left)
        .render(frame, chunk);
}

fn block_default<'a>() -> Block<'a> {
    Block::default()
        .border_style(style_default(Color::Black))
        .borders(Borders::ALL)
}

fn style_default(fg: Color) -> Style {
    Style::default().bg(Color::Black).fg(fg)
}

fn style_bold(fg: Color) -> Style {
    style_default(fg).modifier(Modifier::BOLD)
}
