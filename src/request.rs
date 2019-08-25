use super::settings::Settings;
use chrono::{DateTime, Utc};
use futures::Future;
use reqwest::r#async::Client;
use std::fmt;

#[derive(Debug)]
pub struct Metric {
    pub start_time: DateTime<Utc>,
    pub stop_time: DateTime<Utc>,
    pub status_code: Option<String>,
    pub request_error: Option<String>,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self
            .status_code
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or("".to_string());

        let time = self
            .stop_time
            .signed_duration_since(self.start_time)
            .num_milliseconds();

        let error = self
            .request_error
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or("".to_string());

        write!(f, "[{}] {}ms\t{}", code, time, error)
    }
}

pub fn send(client: &Client, settings: &Settings) -> impl Future<Item = Metric, Error = ()> {
    let method = settings.method.clone();
    let url = settings.url.clone();

    let request = client.request(method, url);

    let request = request.headers(settings.headers.clone());

    let request = match &settings.data {
        Some(data) => request.body(data.clone()),
        None => request,
    };

    let start_time = Utc::now();

    request.send().then(move |result| {
        let stop_time = Utc::now();

        let status_code = match &result {
            Ok(response) => Some(response.status().to_string()),
            Err(_) => None,
        };

        let request_error = match &result {
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        };

        let metric = Metric {
            start_time: start_time,
            stop_time: stop_time,
            status_code: status_code,
            request_error: request_error,
        };

        Ok(metric)
    })
}

pub fn build_client() -> Client {
    Client::new()
}
