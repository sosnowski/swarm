use std::collections::HashMap;

use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::http::{Error};
use hyper::header::{GetAll, HeaderValue};
use hyper::{Client, Request, Method, Uri, Body};
use url::Url;
use tokio::time::{Duration, Instant, sleep, timeout};
use tokio::sync::mpsc::{Sender};
use crate::config::{Schedule, Task, HttpMethod, RequestDetails, RequestData, Body as BodyType, Url as TaskUrl};
use crate::runner::{TaskResult, ErrorType, UserResult};

type CookiesStore = HashMap<String, String>;

// TODO: Store and send cookies per domain!
fn store_cookies(cookies_store: &mut CookiesStore, cookie_headers: GetAll<HeaderValue>) {
    for cookie_header in cookie_headers {
        let cookie_string = cookie_header.to_str().unwrap();
        let mut chunks = cookie_string.split("=");
        let cookie_name = chunks.next();

        if let Some(cookie_name) = cookie_name {
            cookies_store.insert(cookie_name.to_string(), cookie_string.to_string());
        }
    }
}

fn to_hyper_method(method: &HttpMethod) -> Method {
    return match method {
        HttpMethod::GET => Method::GET,
        HttpMethod::POST => Method::POST,
        HttpMethod::PUT => Method::PUT,
        HttpMethod::DELETE => Method::DELETE,
    }
}

fn build_request(url_details: &TaskUrl, method: &HttpMethod, data: &RequestData, cookies_store: &CookiesStore) -> Result<Request<Body>, Error> {
    let mut builder = Request::builder()
        .method(to_hyper_method(method));

    let mut url = url_details.url.clone();
    if url_details.args.len() > 0 {
        if let Some(params) = &data.params {
            for arg in url_details.args.iter() {
                let param_value = if let Some(value) = params.get(arg) {
                    value
                } else {
                    ""
                };

                let replace = format!("{{{v}}}", v = arg);
                url = url.replace(&replace, param_value);
            }
        }
    }
    
    let parsed_url;
    if let Some(query) = &data.query {
        parsed_url = Url::parse_with_params(&url, query.iter()).unwrap();
    } else {
        parsed_url = Url::parse(&url).unwrap();
    }

    if let Some(headers) = &data.headers {
        for (key, value) in headers.iter() {
            builder = builder.header(key, value);
        }
    }

    if cookies_store.len() > 0 {
        let cookie_header = cookies_store.values()
            .map(|cookie| cookie.clone())
            .collect::<Vec<String>>()
            .join("; ");
        
        builder = builder.header("cookie", cookie_header);
    }

    let request_body;
    if let Some(body) = &data.body {
        match &body {
            BodyType::Json(content) => {
                builder = builder.header("content-type", "application/json");
                request_body = Body::from(content.to_string());
            },
            BodyType::Text(content) => {
                builder = builder.header("content-type", "text/plain");
                request_body = Body::from(content.to_string());
            }
        }
    } else {
        request_body = Body::empty();
    }

    let uri: hyper::Uri = parsed_url.to_string().parse()?;
    let req = builder.uri(uri)
        .body(request_body)?;

    return Ok(req);
}

async fn make_request(id: &String, client: &Client<HttpsConnector<HttpConnector>>, mut cookies_store: &mut CookiesStore, request: Request<Body>) -> TaskResult {
    let started_at = Instant::now();
    let url = request.uri().to_string();

    let result = timeout(
        Duration::from_secs(10),
        client.request(request)
    ).await;
    let elapsed: usize = started_at.elapsed().as_millis().try_into().unwrap();

    let task_result: TaskResult = match result {
        Ok(req_result) => match req_result {
                Ok(response) => {
                    store_cookies(&mut cookies_store, response.headers().get_all("set-cookie"));
                    return TaskResult {
                        id: id.clone(),
                        url,
                        duration: elapsed,
                        success: response.status().is_success(),
                        error: !response.status().is_success(),
                        error_type: if response.status().is_client_error() {
                            ErrorType::Request4xx
                        } else if response.status().is_server_error() {
                            ErrorType::Request5xx
                        } else {
                            ErrorType::RequestOther
                        }
                    }
                },
                Err(_) => TaskResult {
                    id: id.clone(),
                    url: url,
                    duration: elapsed,
                    success: false,
                    error: true,
                    error_type: ErrorType::Connection,
                }
        },
        Err(_) => TaskResult {
            id: id.clone(),
            url: url,
            duration: elapsed,
            success: false,
            error: true,
            error_type: ErrorType::Timeout,
        }
    };

    return task_result;
}

pub async fn http_user(schedule: Schedule) -> UserResult {

    let https = HttpsConnector::new();
    let http_client = Client::builder().build::<_, hyper::Body>(https);

    let mut cookies_store: CookiesStore = HashMap::new();

    let mut results = vec![];
    for task in schedule.tasks {
        match task {
            Task::Request(details) => {
                let RequestDetails {
                    url,
                    method,
                    data,
                    repeat
                } = details;

                let task_id = url.url.clone(); // TODO better ID (for example include METHOD)

                let repeat = match repeat {
                    Some(v) => v,
                    None => 1,
                };

                let request_data = match data {
                    Some(data) => data,
                    None => vec![
                        RequestData {
                            params: None,
                            query: None,
                            body: None,
                            headers: None,
                        }
                    ],
                };
                
                for _ in 0..repeat {
                    for data_record in &request_data {
                        let request = build_request(&url, &method, data_record, &cookies_store).unwrap();
                        let result = make_request(&task_id, &http_client, &mut cookies_store, request).await;
                        results.push(result);
                    }
                }
            },
            Task::Wait(duration) => {
                sleep(Duration::from_secs(duration.try_into().unwrap())).await;
            }
        }
    }

    return Ok(results);
}