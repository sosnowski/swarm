use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{Duration, Instant, sleep};
use tokio::task::JoinHandle;

use crate::config::{Config};
use crate::user::http_user;
use super::{TaskResult, StatusMessage, ReportMessage, UrlResults};

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

async fn status_updater(_config: Config, users_counter: Arc<Mutex<usize>>, mut status_receiver: Receiver<StatusMessage>, report_sender: Sender<ReportMessage>) -> () {

    let mut last_send = Instant::now();
    let mut num_of_failed_users: usize = 0;
    let mut aggregated_results: HashMap<String, UrlResults> = HashMap::new();

    while let Some(status) = status_receiver.recv().await {
        match status {
            StatusMessage::UserCreated {} => {
                let mut users_counter = users_counter.lock().unwrap();
                *users_counter += 1;
            },
            StatusMessage::UserFailed {} => {
                num_of_failed_users += 1;
                let mut users_counter = users_counter.lock().unwrap();
                *users_counter -= 1;
            },
            StatusMessage::UserFinished { results } => {
                let num_of_users = {
                    let mut users_counter = users_counter.lock().unwrap();
                    *users_counter -= 1;
                    users_counter.clone()
                };

                aggregate_results(&mut aggregated_results, results);

                if last_send.elapsed().as_secs() >= 10 {
                    //aggregate and send
                    last_send = Instant::now();

                    report_sender.send(ReportMessage::Report {
                        num_of_users,
                        num_of_failed_users,
                        results: aggregated_results.clone()
                    }).await.unwrap();
                }
            }
        }
    }

    let num_of_users = {
        let locked = users_counter.lock().unwrap();
        locked.clone()
    };
    report_sender.send(ReportMessage::Report {
        num_of_users,
        num_of_failed_users,
        results: aggregated_results.clone()
    }).await.unwrap();

    println!("User updater finished");
}

async fn users_manager(config: Config, users_counter: Arc<Mutex<usize>>, status_sender: Sender<StatusMessage>) -> () {
    let started_at = Instant::now();
    let (done_sender, mut done_receiver) = channel::<bool>(1);

    loop {
        sleep(Duration::from_millis(150)).await;
        if started_at.elapsed().as_secs() >= config.duration.try_into().unwrap() {
            break;
        }

        println!("Users Manager Tick!");
        let counter = {
            let users_counter = users_counter.lock().unwrap();
            users_counter.clone()
        };

        if counter < config.users {
            let mut to_add = config.users - counter;
            if to_add > 10 {
                to_add = 10;
            }

            let mut i = 0;
            while i < to_add {
                let schedule = config.schedule.clone();
                let status_sender = status_sender.clone();
                let done_sender = done_sender.clone();
                tokio::spawn(async move {
                    // println!("Adding new user...");
                    http_user(schedule, status_sender, done_sender).await;
                });
                i += 1;
            }
        }
    }

    drop(done_sender);

    println!("Users manager finished, waiting for pending users...");
    done_receiver.recv().await;
    println!("Users manager fully finished");
}

pub struct Runner {
    receiver: Option<Receiver<ReportMessage>>,
    manager_handle: Option<JoinHandle<()>>,
    status_handle: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn start(config: Config) -> Runner {

        let (status_sender, status_receiver) = channel::<StatusMessage>(10000);
        let (report_sender, report_receiver) = channel::<ReportMessage>(100);
        let users_counter = Arc::new(Mutex::new(0));

        let manager_handle = {
            let sender = status_sender.clone();
            let config = config.clone();
            let users_counter = users_counter.clone();
            let handler = tokio::spawn(async move {
                users_manager(config, users_counter, sender).await;
            });
            handler
        };

        let status_handle = {
            let config = config.clone();
            let users_counter = users_counter.clone();
            let handle = tokio::spawn(async move {
                status_updater(config, users_counter, status_receiver, report_sender).await;
            });
            handle
        };


        return Runner {
            receiver: Some(report_receiver),
            status_handle: Some(status_handle),
            manager_handle: Some(manager_handle),
        };
    }

    pub fn take_receiver(&mut self) -> Result<Receiver<ReportMessage>, &str> {
        if let Some(receiver) = self.receiver.take() {
            return Ok(receiver);
        }
        return Err("Runner is either not started or receiver already taken");
    }

    pub async fn wait_until_finished(&mut self) {
        if let Some(manager) = self.manager_handle.take() {
            manager.await.unwrap();
        }
        if let Some(status) = self.status_handle.take() {
            status.await.unwrap();
        }
    }
}
