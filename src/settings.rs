use clap::{value_t, values_t, ArgMatches};
use http::header::{HeaderMap, HeaderName};
use http::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_TYPE, HOST, USER_AGENT};
use http::{Method, Uri};

#[derive(Debug)]
pub struct Settings {
    pub method: Method,
    pub url: Uri,
    pub data: Option<String>,
    pub headers: HeaderMap,

    pub concurrency: u32,
    pub rate: Option<u32>,
    pub total: Option<u32>,
    pub duration: Option<u32>,
}

impl Settings {
    pub fn from_matches(matches: ArgMatches) -> Settings {
        let method = value_t!(matches, "method", Method).expect("method");
        let url = value_t!(matches, "url", Uri).expect("url");
        let data = value_t!(matches, "data", String).ok();
        let headers = Settings::from_matches_headers(&matches, &url);

        let concurrency = value_t!(matches, "concurrency", u32).expect("concurrency");
        let rate = value_t!(matches, "rate", u32).ok();
        let total = value_t!(matches, "total", u32).ok();
        let duration = value_t!(matches, "duration", u32).ok();

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

    fn from_matches_headers(matches: &ArgMatches, url: &Uri) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // Default headers
        headers.insert(ACCEPT, "*/*".parse().expect("header"));
        headers.insert(ACCEPT_ENCODING, "gzip, deflate".parse().expect("header"));
        headers.insert(USER_AGENT, "http-storm/0.1.0".parse().expect("header"));
        headers.insert(HOST, url.host().expect("host").parse().expect("header"));

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
