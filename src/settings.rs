use clap::{value_t, values_t, ArgMatches};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_TYPE, HOST, USER_AGENT};
use reqwest::{Method, Url};

#[derive(Debug, Clone)]
pub struct Settings {
    pub method: Method,
    pub url: Url,
    pub data: Option<String>,
    pub headers: HeaderMap,

    pub concurrency: usize,
    pub rate: Option<u64>,
    pub total: Option<u64>,
    pub duration: Option<u64>,
}

impl Settings {
    pub fn from_matches(matches: ArgMatches) -> Settings {
        let method = value_t!(matches, "method", Method).expect("method");
        let url = value_t!(matches, "url", Url).expect("url");
        let data = value_t!(matches, "data", String).ok();
        let headers = Settings::from_matches_headers(&matches, &url);

        let concurrency = value_t!(matches, "concurrency", usize).expect("concurrency");
        let rate = value_t!(matches, "rate", u64).ok();
        let total = value_t!(matches, "total", u64).ok();
        let duration = value_t!(matches, "duration", u64).ok();

        Settings {
            method: method,
            url: url,
            data: data,
            headers: headers,

            concurrency: concurrency,
            rate: rate,
            total: total,
            duration: duration,
        }
    }

    fn from_matches_headers(matches: &ArgMatches, url: &Url) -> HeaderMap {
        let host = url.host().expect("host");

        let mut headers = HeaderMap::new();

        // Default headers
        headers.insert(ACCEPT, "*/*".parse().expect("header"));
        headers.insert(ACCEPT_ENCODING, "gzip, deflate".parse().expect("header"));
        headers.insert(USER_AGENT, "http-storm/0.1.0".parse().expect("header"));
        headers.insert(HOST, host.to_string().parse().expect("host"));

        Settings::from_matches_headers_json(matches, &mut headers);
        Settings::from_matches_headers_form(matches, &mut headers);
        Settings::from_matches_headers_custom(matches, &mut headers);

        headers
    }

    fn from_matches_headers_json(matches: &ArgMatches, headers: &mut HeaderMap) -> () {
        let json = matches.is_present("json");
        let accept = "application/json, */*";
        let content_type = "application/json";

        if json {
            headers.insert(ACCEPT, accept.parse().expect("header"));
            headers.insert(CONTENT_TYPE, content_type.parse().expect("header"));
        }
    }

    fn from_matches_headers_form(matches: &ArgMatches, headers: &mut HeaderMap) -> () {
        let form = matches.is_present("form");
        let content_type = "application/x-www-form-urlencoded; charset=utf-8";

        if form {
            headers.insert(CONTENT_TYPE, content_type.parse().expect("header"));
        }
    }

    fn from_matches_headers_custom(matches: &ArgMatches, headers: &mut HeaderMap) -> () {
        let header = values_t!(matches, "header", String).ok();

        header.map(|values| {
            for pair in values.chunks(2) {
                match pair {
                    [key, value] => {
                        let name = key.parse::<HeaderName>().expect("header");
                        let value = value.parse().expect("header");

                        headers.insert(name, value);
                    }
                    _ => unreachable!(),
                }
            }
        });
    }
}
