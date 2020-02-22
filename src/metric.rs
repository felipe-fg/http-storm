use super::settings::Settings;
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use std::fmt;

#[derive(Debug)]
pub struct Metric {
    pub start_time: DateTime<Utc>,
    pub stop_time: DateTime<Utc>,
    pub duration: Duration,
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

impl Metric {
    pub async fn from_request(client: &Client, settings: &Settings) -> Metric {
        let method = settings.method.clone();
        let url = settings.url.clone();

        let request = client.request(method, url);

        let request = request.headers(settings.headers.clone());

        let request = match &settings.data {
            Some(data) => request.body(data.clone()),
            None => request,
        };

        let start_time = Utc::now();

        let result = request.send().await;

        let stop_time = Utc::now();

        let duration = stop_time.signed_duration_since(start_time);

        let status_code = match &result {
            Ok(response) => Some(response.status().to_string()),
            Err(_) => None,
        };

        let request_error = match &result {
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        };

        Metric {
            start_time: start_time,
            stop_time: stop_time,
            duration: duration,
            status_code: status_code,
            request_error: request_error,
        }
    }
}
