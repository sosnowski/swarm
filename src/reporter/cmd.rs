use tokio::sync::mpsc::{Receiver};
use tokio::task::{JoinHandle};
use crate::runner::{ReportMessage, ErrorType};

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

async fn logger(_config: Config, mut report_receiver: Receiver<ReportMessage>) -> () {

    while let Some(report) = report_receiver.recv().await {
        match report {
            ReportMessage::Report {
                num_of_failed_users,
                num_of_users,
                results
            } => {
                println!("================== REPORT ==================");
                println!("Number of users: {}, failed users: {}", num_of_users, num_of_failed_users);
                for (id, result) in results.iter() {
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
        }
    }

    println!("Logger is finished");
}

pub struct CmdReporter {
    config: Config,
    logger: Option<JoinHandle<()>>,
}

impl CmdReporter {
    pub fn start(config: Config, runner_receiver: Receiver<ReportMessage>) -> CmdReporter {

        let logger_handle = {
            let config = config.clone();
            tokio::spawn(async move {
                logger(config, runner_receiver).await;
            })
        };

        return CmdReporter {
            config,
            logger: Some(logger_handle),
        };
    }

    pub async fn wait_until_finished(&mut self) -> () {
        if let Some (logger) = self.logger.take() {
            logger.await.unwrap();
        }
    }
}