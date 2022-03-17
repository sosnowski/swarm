use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::io;
use tokio::sync::mpsc::{Receiver};
use tokio::time::{Duration, interval, sleep};
use tokio::task::{JoinHandle};
use console::Term;
use crate::runner::{ReportMessage, ErrorType, TaskResult, UrlResults};

use crate::config::Config;

struct AggregatedResults {
    num_of_failed_users: usize,
    current_users: usize,
    duration: usize,

    url_results: HashMap<String, UrlResults>,
}

impl AggregatedResults {
    fn new() -> AggregatedResults {
        return AggregatedResults {
            num_of_failed_users: 0,
            current_users: 0,
            duration: 0,
            url_results: HashMap::new(),
        };
    }
}

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
struct Terminal {
    count_lines: usize,
    term: Term,
}

impl Terminal {
    fn new() -> Terminal {
        return Terminal { count_lines: 0, term: Term::stdout() };
    }

    fn log_results(&mut self, results: &AggregatedResults) -> io::Result<()> {
        let term = &self.term;
        term.write_line("================== REPORT ==================")?;
        term.write_line(&format!("Number of users: {}, failed users: {}", results.current_users, results.num_of_failed_users))?;
        term.write_line(&format!("Duration: {}", results.duration))?;

        for (id, result) in results.url_results.iter() {
            term.write_line(&format!("\t ID: {}", id))?;
            term.write_line(&format!("\t\t Number of requests: {}", result.num_of_requests))?;
            term.write_line(&format!("\t\t Number of errors: {}", result.num_of_errors))?;
            for (err_type, counter) in result.error_types.iter() {
                term.write_line(&format!("\t\t\t{} errror: {}", print_error_type(err_type), counter))?;
                self.count_lines += 1;
            }
            term.write_line(&format!("\t\t Average duration: {}", result.average_duration))?;
            self.count_lines += 4;
        }
        term.write_line("=============================================")?;
        self.count_lines += 4;
        return Ok(());
    }

    fn clear_results(&mut self) -> io::Result<()> {
        if self.count_lines > 0 {
            self.term.clear_last_lines(self.count_lines)?;
            self.count_lines = 0;
        }
        return Ok(());
    }
}

fn aggregate_results(url_results: &mut HashMap<String, UrlResults>, results: Vec<TaskResult>) -> () {
    
    for result in results.into_iter() {
        let entry = url_results.entry(result.id).or_insert(UrlResults {
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

async fn aggregator(_config: Config, mut report_receiver: Receiver<ReportMessage>) -> () {
    let mut aggregated_results = AggregatedResults::new();
    let mut terminal = Terminal::new();

    let mut interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                terminal.clear_results().unwrap();
                terminal.log_results(&aggregated_results).unwrap();
            },
            msg = report_receiver.recv() => {
                match msg {
                    Some (report_msg) => {
                        aggregated_results.current_users = report_msg.current_users;
                        aggregated_results.duration = report_msg.duration;

                        for task_result in report_msg.results.into_iter() {
                            match task_result {
                                Ok(res) => {
                                    aggregate_results(&mut aggregated_results.url_results, res);
                                },
                                Err(msg) => {
                                    aggregated_results.num_of_failed_users += 1;
                                }
                            }
                        }
                    },
                    None => {
                        terminal.clear_results().unwrap();
                        terminal.log_results(&aggregated_results).unwrap();
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