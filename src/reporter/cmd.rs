use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use tokio::sync::mpsc::{Receiver};
use tokio::time::{Duration, interval, sleep};
use tokio::task::{JoinHandle};
use crate::runner::{ReportMessage, ErrorType, TaskResult, UrlResults};

use crate::config::Config;

fn print_error_type(err_type: &ErrorType) -> &'static str {
    return match err_type {
        ErrorType::Connection => "Connection",
        ErrorType::Internal => "Internal application",
        ErrorType::RequestOther => "Other",
        ErrorType::Request4xx => "4XX",
        ErrorType::Request5xx => "5XX",
        ErrorType::Timeout => "Timeout",
    };
}

fn aggregate_results(aggregated_results: &mut HashMap<String, UrlResults>, results: Vec<TaskResult>) -> () {
    
    for result in results.into_iter() {
        let entry = aggregated_results.entry(result.id).or_insert(UrlResults {
            num_of_requests: 0,
            num_of_errors: 0,
            average_duration: 0,
            error_types: HashMap::new(),
        });

        entry.num_of_requests += 1;
        if result.error {
            entry.num_of_errors += 1;
            let error_type_counter = entry.error_types.entry(result.error_type).or_insert(0);
            *error_type_counter += 1;
        } else {
            entry.average_duration = entry.average_duration + ((result.duration - entry.average_duration) / (entry.num_of_requests as isize - entry.num_of_errors as isize));
        }
    }
}

fn log_results(current_users: usize, num_of_failed_users: usize, duration: usize, aggregated_results: &HashMap<String, UrlResults>) -> () {
    println!("================== REPORT ==================");
    println!("Number of users: {}, failed users: {}", current_users, num_of_failed_users);
    println!("Duration: {}", duration);
    for (id, result) in aggregated_results.iter() {
        println!("\t ID: {}", id);
        println!("\t\t Number of requests: {}", result.num_of_requests);
        println!("\t\t Number of errors: {}", result.num_of_errors);
        for (err_type, counter) in result.error_types.iter() {
            println!("\t\t\t{} errror: {}", print_error_type(err_type), counter);
        }
        println!("\t\t Average duration: {}", result.average_duration);
    }
    println!("=============================================")
}

async fn aggregator(_config: Config, mut report_receiver: Receiver<ReportMessage>) -> () {
    let mut num_of_failed_users = 0;
    let mut current_users = 0;
    let mut duration = 0;
    let mut aggregated_results = HashMap::<String, UrlResults>::new();

    let mut interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                log_results(current_users, num_of_failed_users, duration, &aggregated_results);
            },
            msg = report_receiver.recv() => {
                match msg {
                    Some (report_msg) => {
                        current_users = report_msg.current_users;
                        duration = report_msg.duration;
                        for task_result in report_msg.results.into_iter() {
                            match task_result {
                                Ok(res) => {
                                    aggregate_results(&mut aggregated_results, res);
                                },
                                Err(msg) => {
                                    num_of_failed_users += 1;
                                }
                            }
                        }
                    },
                    None => {
                        log_results(current_users, num_of_failed_users, duration, &aggregated_results);
                        break;
                    }
                }
            }
        }
    }
}

pub struct CmdReporter {
    config: Config,
    aggregator: Option<JoinHandle<()>>,
}

impl CmdReporter {
    pub fn start(config: Config, runner_receiver: Receiver<ReportMessage>) -> CmdReporter {

        let aggregator_handle = {
            let config = config.clone();
            tokio::spawn(async move {
                aggregator(config, runner_receiver).await;
            })
        };

        return CmdReporter {
            config,
            aggregator: Some(aggregator_handle),
        };
    }

    pub async fn wait_until_finished(&mut self) -> () {
        if let Some (aggregator) = self.aggregator.take() {
            aggregator.await.unwrap();
        }
    }
}