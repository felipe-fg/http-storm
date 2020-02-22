use clap::{load_yaml, App, AppSettings};
use metric::Metric;
use settings::Settings;
use tokio::sync::mpsc;

mod metric;
mod settings;
mod storm;
mod ui;

#[tokio::main]
async fn main() {
    let cli = load_yaml!("cli.yml");

    let matches = App::from_yaml(cli)
        .setting(AppSettings::DeriveDisplayOrder)
        .get_matches();

    let settings = Settings::from_matches(matches);

    let (metric_sender, metric_receiver) = mpsc::unbounded_channel::<Metric>();

    storm::run(&settings, metric_sender);

    ui::render(&settings, metric_receiver).await;
}
