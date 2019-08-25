use super::request;
use super::request::Metric;
use super::Settings;
use chrono::{DateTime, Utc};
use futures::future::lazy;
use futures::sync::mpsc;
use futures::sync::mpsc::Sender;
use futures::{Future, Sink, Stream};
use reqwest::r#async::Client;

pub fn run(settings: Settings) -> () {
    tokio::run(lazy(|| {
        let client = request::build_client();
        let (metric_sender, metric_receiver) = mpsc::channel(1_024);
        let mut total_sent = settings.concurrency as u32;
        let start_time = Utc::now();

        for _ in 0..settings.concurrency {
            send_request(settings.clone(), client.clone(), metric_sender.clone());
        }

        metric_receiver.for_each(move |metric| {
            println!("{}", metric);

            if !should_stop(&settings, start_time, total_sent) {
                total_sent += 1;

                send_request(settings.clone(), client.clone(), metric_sender.clone());
            }

            Ok(())
        })
    }));
}

fn should_stop(settings: &Settings, start_time: DateTime<Utc>, total_sent: u32) -> bool {
    let running_seconds = Utc::now().signed_duration_since(start_time).num_seconds();

    let time_exceeded = settings
        .duration
        .map(|duration| running_seconds >= duration.into())
        .unwrap_or(false);

    let count_exceeded = settings
        .total
        .map(|total| total_sent >= total)
        .unwrap_or(false);

    time_exceeded || count_exceeded
}

fn send_request(settings: Settings, client: Client, metric_sender: Sender<Metric>) {
    tokio::spawn(lazy(move || {
        request::send(&client, &settings)
            .and_then(|metric| metric_sender.send(metric).map(|_| {}).map_err(|_| {}))
            .map(|_| {})
            .map_err(|_| {})
    }));
}
