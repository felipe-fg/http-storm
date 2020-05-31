use crate::settings::Settings;
use crate::summary;
use crate::view;
use crate::worker::{WorkerCommand, WorkerMessage};
use std::io::{self, stdout, Bytes, Read};
use std::time;
use termion::async_stdin;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::AsyncReader;
use tokio::{self, sync::mpsc, sync::watch};
use tui::backend::{Backend, TermionBackend};
use tui::Terminal;

#[derive(Debug)]
enum InputCommand {
    Stop,
    Quit,
    None,
}

impl InputCommand {
    fn from_input(input: io::Result<u8>) -> InputCommand {
        if let Ok(key) = input {
            match key {
                b'q' | b'Q' => InputCommand::Quit,
                b's' | b'S' => InputCommand::Stop,
                _ => InputCommand::None,
            }
        } else {
            InputCommand::None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum ViewStatus {
    Running,
    Finished,
}

pub async fn render(
    settings: &Settings,
    mut receiver: mpsc::UnboundedReceiver<WorkerMessage>,
    sender: watch::Sender<WorkerCommand>,
) -> () {
    let mut terminal = create_terminal().expect("terminal");
    let mut terminal_stdin = create_terminal_stdin();

    let mut messages = Vec::new();
    let mut summary = summary::compute(&messages, settings, messages.len());

    let mut finished_workers = 0;
    let mut current_status = ViewStatus::Running;
    let mut previous_status = ViewStatus::Running;

    'render: loop {
        while let Ok(message) = receiver.try_recv() {
            if message.finished {
                finished_workers += 1;
            }

            messages.push(message);
        }

        if finished_workers == settings.concurrency {
            current_status = ViewStatus::Finished;
        }

        'input: while let Some(input) = terminal_stdin.next() {
            match InputCommand::from_input(input) {
                InputCommand::Stop => {
                    current_status = ViewStatus::Finished;
                    break 'input;
                }
                InputCommand::Quit => {
                    break 'render;
                }
                _ => (),
            };
        }

        if current_status == ViewStatus::Finished && previous_status != ViewStatus::Finished {
            receiver.close();

            match sender.broadcast(WorkerCommand::Stop) {
                _ => (),
            };

            summary = summary::compute(&messages, settings, messages.len());
        } else if current_status == ViewStatus::Running {
            let border = 2;
            let size = terminal.get_frame().size();
            let columns = (size.width - border) as usize;

            summary = summary::compute(&messages, settings, columns);
        }

        terminal
            .draw(|frame| {
                match current_status {
                    ViewStatus::Running => view::draw_running(&summary, frame),
                    ViewStatus::Finished => view::draw_finished(&summary, frame),
                };
            })
            .expect("draw");

        previous_status = current_status;

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
