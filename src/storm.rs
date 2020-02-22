use super::metric::Metric;
use super::settings::Settings;
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time;

pub fn run(settings: &Settings, metric_sender: UnboundedSender<Metric>) -> () {
    let client = Client::new();

    for _ in 1..settings.concurrency {
        spawn_worker(&client, settings, metric_sender.clone());
    }

    spawn_worker(&client, settings, metric_sender);
}

fn spawn_worker(client: &Client, settings: &Settings, sender: UnboundedSender<Metric>) -> () {
    let client = client.clone();
    let settings = settings.clone();

    let workers = settings.concurrency as f64;
    let worker_rate = settings.rate.map(|x| x as f64 / workers);
    let worker_total = settings.total.map(|x| x as f64 / workers);
    let worker_duration = settings.duration;

    tokio::spawn(async move {
        let worker_start_time = Utc::now();
        let mut worker_count = 0 as u64;

        loop {
            let metric = Metric::from_request(&client, &settings).await;
            let metric_start_time = metric.start_time;
            let metric_stop_time = metric.stop_time;

            sender.send(metric).expect("metric");

            worker_count += 1;

            if !total_check(worker_total, worker_count) {
                break;
            }

            if !duration_check(worker_duration, worker_start_time) {
                break;
            }

            rate_delay(worker_rate, metric_start_time, metric_stop_time).await;
        }
    });
}

fn total_check(total: Option<f64>, count: u64) -> bool {
    match total {
        Some(total) => count < total.ceil() as u64,
        None => true,
    }
}

fn duration_check(duration: Option<u32>, start: DateTime<Utc>) -> bool {
    match duration {
        Some(duration) => {
            let duration = Duration::from_secs(duration as u64);

            let elapsed = Utc::now()
                .signed_duration_since(start)
                .to_std()
                .expect("elapsed");

            elapsed < duration
        }
        None => true,
    }
}

async fn rate_delay(rate: Option<f64>, start: DateTime<Utc>, stop: DateTime<Utc>) -> () {
    if let Some(rate) = rate {
        let rate_duration = Duration::from_secs(1).div_f64(rate);

        let metric_duration = stop
            .signed_duration_since(start)
            .to_std()
            .expect("metric duration");

        if metric_duration < rate_duration {
            let delay_duration = rate_duration - metric_duration;

            time::delay_for(delay_duration).await;
        }
    }
}
