use crate::settings::Settings;
use crate::stats::{self, Stats};
use crate::worker::WorkerMessage;

#[derive(Debug)]
pub struct Summary {
    pub request_method: String,
    pub request_url: String,

    pub elapsed_seconds: u64,
    pub total_count: u64,
    pub progress_percent: u16,

    pub stats: Stats,
}

pub fn compute(messages: &[WorkerMessage], settings: &Settings, take: usize) -> Summary {
    let request_method = request_method(settings);
    let request_url = request_url(settings);

    let elapsed_seconds = elapsed_seconds(messages);
    let total_count = total_count(messages);
    let progress_percent = progress_percent(settings, elapsed_seconds, total_count);

    let stats = stats(messages, take);

    Summary {
        request_method,
        request_url,

        elapsed_seconds,
        total_count,
        progress_percent,

        stats,
    }
}

fn request_method(settings: &Settings) -> String {
    settings.method.to_string()
}

fn request_url(settings: &Settings) -> String {
    settings.url.to_string()
}

fn elapsed_seconds(messages: &[WorkerMessage]) -> u64 {
    let last_message = messages.iter().rev().find(|message| message.id == 1);

    if let Some(message) = last_message {
        message.elapsed_time.num_seconds() as u64
    } else {
        0
    }
}

fn total_count(messages: &[WorkerMessage]) -> u64 {
    messages.len() as u64
}

fn progress_percent(settings: &Settings, elapsed_seconds: u64, total_count: u64) -> u16 {
    let total_ratio = settings
        .total
        .map(|total| total_count as f64 / total as f64)
        .unwrap_or(0f64);

    let duration_ratio = settings
        .duration
        .map(|duration| elapsed_seconds as f64 / duration as f64)
        .unwrap_or(0f64);

    let ratio = total_ratio.max(duration_ratio);

    (ratio * 100f64).ceil().min(100f64) as u16
}

fn stats(messages: &[WorkerMessage], take: usize) -> Stats {
    let from = messages.len() as i64 - take as i64;
    let from = from.max(0) as usize;

    let page = &messages[from..];

    stats::compute(page)
}
