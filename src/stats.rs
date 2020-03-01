use crate::worker::WorkerMessage;
use average::{concatenate, define_histogram, Estimate, Max, Min, Variance};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Stats {
    pub count: u64,
    pub status: Vec<(String, u64)>,

    pub time_values: Vec<u64>,
    pub time_minimum: u64,
    pub time_mean: u64,
    pub time_maximum: u64,
    pub time_stddev: u64,
    pub time_histogram: Vec<((u64, u64), u64)>,
}

concatenate!(
    Estimator,
    [Min, min, min],
    [Max, max, max],
    [Variance, variance, mean, population_variance]
);

define_histogram!(hist, 10);

pub fn compute(messages: &[WorkerMessage]) -> Stats {
    let count = count(messages);
    let status = status(messages);

    let time_values = time_values(messages);
    let estimator = estimator(&time_values);

    let time_minimum = time_minimum(&estimator);
    let time_mean = time_mean(&estimator);
    let time_maximum = time_maximum(&estimator);
    let time_stddev = time_stddev(&estimator);

    let time_histogram = time_histogram(&time_values, time_minimum, time_maximum);

    Stats {
        count,
        status,

        time_values,
        time_minimum,
        time_mean,
        time_maximum,
        time_stddev,
        time_histogram,
    }
}

fn count(messages: &[WorkerMessage]) -> u64 {
    messages.len() as u64
}

fn status(messages: &[WorkerMessage]) -> Vec<(String, u64)> {
    let mut map = HashMap::<String, u64>::new();

    for message in messages {
        let status = message
            .metric
            .status_code
            .as_ref()
            .map(|status_code| status_code.to_string())
            .unwrap_or("000".to_string());

        let count = map.get(&status).map(|value| *value).unwrap_or(0u64);

        map.insert(status, count + 1);
    }

    map.into_iter().collect()
}

fn time_values(messages: &[WorkerMessage]) -> Vec<u64> {
    messages
        .iter()
        .map(|message| message.metric.elapsed_time.num_milliseconds() as u64)
        .collect()
}

fn estimator(values: &[u64]) -> Estimator {
    values.iter().map(|value| *value as f64).collect()
}

fn time_histogram(values: &[u64], min: u64, max: u64) -> Vec<((u64, u64), u64)> {
    let mut histogram = hist::Histogram::with_const_width(min as f64 - 1f64, max as f64 + 1f64);

    for value in values {
        histogram.add(*value as f64).expect("histogram bounds");
    }

    histogram
        .iter()
        .map(|((lower, upper), count)| ((lower.floor() as u64, upper.floor() as u64), count as u64))
        .collect()
}

fn time_minimum(estimator: &Estimator) -> u64 {
    estimator.min().floor() as u64
}

fn time_mean(estimator: &Estimator) -> u64 {
    estimator.mean().floor() as u64
}

fn time_maximum(estimator: &Estimator) -> u64 {
    estimator.max().floor() as u64
}

fn time_stddev(estimator: &Estimator) -> u64 {
    estimator.population_variance().sqrt().floor() as u64
}
