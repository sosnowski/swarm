use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Body {
    Json(String),
    Text(String),
}

#[derive(Clone, Debug)]
pub struct RequestData {
    pub params: Option<HashMap<String, String>>,
    pub query: Option<HashMap<String, String>>,
    pub body: Option<Body>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug)]
pub struct Url {
    pub url: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RequestDetails {
    pub url: Url,
    pub method: HttpMethod,
    pub data: Vec<RequestData>,

}

#[derive(Clone, Debug)]
pub enum Task {
    Request(RequestDetails),
    Wait(usize)
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE
}



#[derive(Clone, Debug)]
pub struct Config {
    pub duration: usize,
    pub users: usize,
    pub warmup: usize,
    pub schedule: Schedule,
}

impl Config {
    pub fn new() -> Config {
        return Config {
            duration: 10,
            users: 1,
            warmup: 0,
            schedule: Schedule {
                tasks: vec![
                    Task::Request(RequestDetails {
                        method: HttpMethod::GET,
                        url: Url {
                            url: "http://localhost:3000/test_get/{param1}/{param2}".to_string(),
                            args: vec![
                                "param1".to_string(),
                                "param2".to_string()
                            ],
                        },
                        data: vec![
                            RequestData {
                                params: Some(HashMap::from([
                                    ("param1".to_string(), "aaaaaaaaaaaaaa11111".to_string()),
                                    ("param2".to_string(), "bbbbbbbbbbbbbb11111".to_string()),
                                ])),
                                headers: Some(HashMap::from([
                                    ("user-agent".to_string(), "awesome rust swarm".to_string()),
                                    ("x-custom-header".to_string(), "custom-header-value".to_string())
                                ])),
                                query: Some(HashMap::from([
                                    ("key1".to_string(), "value1".to_string()),
                                    ("key2".to_string(), "value2".to_string()),
                                ])),
                                body: None,
                            },
                            RequestData {
                                params: Some(HashMap::from([
                                    ("param1".to_string(), "aaaaaaaaaaaaaa222222".to_string()),
                                    ("param2".to_string(), "bbbbbbbbbbbbbb222222".to_string()),
                                ])),
                                headers: Some(HashMap::from([
                                    ("user-agent".to_string(), "awesome rust swarm".to_string()),
                                    ("x-custom-header".to_string(), "custom-header-value".to_string())
                                ])),
                                query: Some(HashMap::from([
                                    ("key1".to_string(), "value1".to_string()),
                                    ("key2".to_string(), "value2".to_string()),
                                ])),
                                body: None,
                            },
                        ]
                    }),
                    // Task::Request(RequestDetails {
                    //     method: HttpMethod::POST,
                    //     url: Url {
                    //         url: "http://localhost:3000/test_post".to_string(),
                    //         args: vec![],
                    //     },
                    //     data: RequestData {
                    //         params: None,
                    //         headers: Some(HashMap::from([
                    //             ("user-agent".to_string(), "awesome rust swarm".to_string()),
                    //             ("x-custom-header".to_string(), "custom-header-value".to_string())
                    //         ])),
                    //         query: Some(HashMap::from([
                    //             ("key1".to_string(), "value1".to_string()),
                    //             ("key2".to_string(), "value2".to_string()),
                    //         ])),
                    //         body: Some(Body::Json(r#"{"body-key": "body-value"}"#.to_string())),
                    //     }
                    // })
                ]
            }
        };
    }
}