use super::settings::Settings;
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use std::fmt;

#[derive(Debug)]
pub struct RequestMetric {
    pub start_time: DateTime<Utc>,
    pub stop_time: DateTime<Utc>,
    pub elapsed_time: Duration,
    pub status_code: Option<String>,
    pub error_message: Option<String>,
}

impl fmt::Display for RequestMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self
            .status_code
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or("000".to_string());

        let time = self.elapsed_time.num_milliseconds();

        let error = self
            .error_message
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or("".to_string());

        write!(f, "[{}] {:>3}ms\t{}", code, time, error)
    }
}

impl RequestMetric {
    pub async fn collect_metric(client: &Client, settings: &Settings) -> RequestMetric {
        let method = settings.method.clone();
        let url = settings.url.clone();

        let request = client.request(method, url);

        let request = request.headers(settings.headers.clone());

        let request = match &settings.data {
            Some(data) => request.body(data.to_string()),
            None => request,
        };

        let start_time = Utc::now();

        let result = request.send().await;

        let stop_time = Utc::now();

        let elapsed_time = stop_time.signed_duration_since(start_time);

        let status_code = match &result {
            Ok(response) => Some(response.status().to_string()),
            Err(_) => None,
        };

        let error_message = match &result {
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        };

        RequestMetric {
            start_time: start_time,
            stop_time: stop_time,
            elapsed_time: elapsed_time,
            status_code: status_code,
            error_message: error_message,
        }
    }
}
