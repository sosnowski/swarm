use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Request5xx,
    Request4xx,
    RequestOther,
    Timeout,
    Connection,
    Internal,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub id: String,
    pub url: String,
    pub success: bool,
    pub error: bool,
    pub error_type: ErrorType,
    pub duration: isize,
}

#[derive(Clone, Debug)]
pub enum ReportMessage {
    Report {
        num_of_users: usize,
        num_of_failed_users: usize,
        results: HashMap<String, UrlResults>,
    }
}

#[derive(Clone, Debug)]
pub struct UrlResults {
    pub num_of_requests: usize,
    pub average_duration: isize,
    pub num_of_errors: usize,
    pub error_types: HashMap<ErrorType, usize>,
}

#[derive(Clone, Debug)]
pub enum StatusMessage {
    UserCreated {},
    UserFailed {},
    UserFinished {
        results: Vec<TaskResult>,
    }
}