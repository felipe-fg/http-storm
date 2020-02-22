use clap::{load_yaml, App, AppSettings};
use settings::Settings;

mod metric;
mod settings;
mod storm;

#[tokio::main]
async fn main() {
    let cli = load_yaml!("cli.yml");

    let matches = App::from_yaml(cli)
        .setting(AppSettings::DeriveDisplayOrder)
        .get_matches();

    let settings = Settings::from_matches(matches);

    storm::run(settings).await;
}
