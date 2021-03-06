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
    pub duration: usize,
}

pub type UserResult = Result<Vec<TaskResult>, String>;

#[derive(Clone, Debug)]
pub struct ReportMessage {
    pub current_users: usize,
    pub results: Vec<UserResult>,
    pub duration: usize,
}

#[derive(Clone, Debug)]
pub enum UserStatus {
    Created,
    Finished(UserResult),
}