use crate::metric::RequestMetric;
use crate::settings::Settings;
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use std::fmt;
use std::time;
use tokio;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WorkerCommand {
    Run,
    Stop,
}

#[derive(Debug)]
pub struct WorkerMessage {
    pub id: usize,
    pub finished: bool,
    pub start_time: DateTime<Utc>,
    pub current_time: DateTime<Utc>,
    pub elapsed_time: Duration,
    pub metric: RequestMetric,
}

impl fmt::Display for WorkerMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Worker {:>3}: {}", self.id, self.metric)
    }
}

pub fn collect_metrics(
    settings: &Settings,
    sender: mpsc::UnboundedSender<WorkerMessage>,
    receiver: watch::Receiver<WorkerCommand>,
) -> () {
    for id in 1..settings.concurrency {
        spawn_worker(settings.clone(), id, sender.clone(), receiver.clone());
    }

    spawn_worker(settings.clone(), settings.concurrency, sender, receiver);
}

fn spawn_worker(
    settings: Settings,
    id: usize,
    sender: mpsc::UnboundedSender<WorkerMessage>,
    receiver: watch::Receiver<WorkerCommand>,
) -> () {
    tokio::spawn(async move {
        let client = Client::new();

        let workers = settings.concurrency as f64;
        let worker_rate = settings.rate.map(|rate| rate as f64 / workers);
        let worker_total = settings.total.map(|total| total as f64 / workers);
        let worker_duration = settings.duration;

        let start_time = Utc::now();

        for count in 1u64.. {
            let command = { *receiver.borrow() };

            if command == WorkerCommand::Stop {
                break;
            }

            let metric = RequestMetric::collect_metric(&client, &settings).await;
            let metric_elapsed_time = metric.elapsed_time;

            let current_time = Utc::now();
            let elapsed_time = current_time.signed_duration_since(start_time);

            let mut finished = false;

            if !total_check(worker_total, count) {
                finished = true;
            }

            if !duration_check(worker_duration, elapsed_time) {
                finished = true;
            }

            let message = WorkerMessage {
                id: id,
                start_time: start_time,
                current_time: current_time,
                elapsed_time: elapsed_time,
                metric: metric,
                finished: finished,
            };

            match sender.send(message) {
                Ok(_) => (),
                Err(_) => break,
            };

            if finished {
                break;
            }

            rate_delay(worker_rate, metric_elapsed_time).await;
        }
    });
}

fn total_check(total: Option<f64>, count: u64) -> bool {
    match total {
        Some(total) => count < total.ceil() as u64,
        None => true,
    }
}

fn duration_check(duration: Option<u64>, elapsed_time: Duration) -> bool {
    match duration {
        Some(duration) => elapsed_time < Duration::seconds(duration as i64),
        None => true,
    }
}

async fn rate_delay(rate: Option<f64>, elapsed_time: Duration) -> () {
    if let Some(rate) = rate {
        let rate_time = time::Duration::from_secs(1).div_f64(rate);

        let metric_time = elapsed_time.to_std().expect("metric time");

        if metric_time < rate_time {
            let delay_time = rate_time - metric_time;

            tokio::time::delay_for(delay_time).await;
        }
    }
}
