use crate::settings::Settings;
use crate::worker::WorkerMessage;
use clap::{load_yaml, App, AppSettings};
use tokio::sync::mpsc;

mod metric;
mod settings;
mod ui;
mod worker;

#[tokio::main(core_threads = 32, max_threads = 1024)]
async fn main() {
    let cli = load_yaml!("cli.yml");

    let matches = App::from_yaml(cli)
        .setting(AppSettings::DeriveDisplayOrder)
        .get_matches();

    let settings = Settings::from_matches(matches);

    let (message_sender, message_receiver) = mpsc::unbounded_channel::<WorkerMessage>();

    worker::collect_metrics(&settings, message_sender);

    ui::render(&settings, message_receiver).await;
}
