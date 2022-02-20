use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{Duration, Instant, interval};
use tokio::task::JoinHandle;

use crate::config::{Config};
use crate::runner::UserResult;
use crate::user::http_user;
use super::{ReportMessage, UserStatus};

fn manage_users(config: &Config, users_counter: usize, status_sender: &Sender<UserStatus>, done_sender: &Sender<bool>) -> () {
    if users_counter < config.users {
        let mut to_add = config.users - users_counter;
        if to_add > 20 {
            to_add = 20;
        }

        let mut i = 0;
        while i < to_add {
            let schedule = config.schedule.clone();
            let status_sender = status_sender.clone();
            let done_sender = done_sender.clone();
            tokio::spawn(async move {
                status_sender.send(UserStatus::Created).await.unwrap();
                let user_result = http_user(schedule, done_sender).await;
                status_sender.send(UserStatus::Finished(user_result)).await.unwrap();
            });
            i += 1;
        }
    }
}

async fn runner(config: Config, report_sender: Sender<ReportMessage>) -> () {
    let (status_sender, mut status_receiver) = channel::<UserStatus>(1000);
    let (done_sender, mut done_receiver) = channel::<bool>(1);

    let started_at = Instant::now();
    let mut interval = interval(Duration::from_millis(200));

    let mut users_counter: usize = 0;
    let mut queued_results: Vec<UserResult> = vec![];

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // send aggregated results
                println!("Sending report...");
                report_sender.send(ReportMessage {
                    current_users: users_counter,
                    results: queued_results.clone(),
                    duration: started_at.elapsed().as_secs().try_into().unwrap(),
                }).await.unwrap();

                queued_results.clear();

                if started_at.elapsed().as_secs() <= config.duration.try_into().unwrap() {
                    manage_users(&config, users_counter, &status_sender, &done_sender);
                } else {
                    println!("Time out, waiting for users to finish....");

                    if users_counter == 0 {
                        //wait till all users finish
                        println!("All users are done");
                        break;
                    }
                    
                }
            },
            msg = status_receiver.recv() => {
                match msg {
                    Some(user_status) => {
                        // receive user status, update counter, aggregate results
                        match user_status {
                            UserStatus::Created => {
                                users_counter += 1;
                            },
                            UserStatus::Finished(result) => {
                                println!("User is finished");
                                users_counter -= 1;
                                queued_results.push(result);
                            },
                        }
                    },
                    None => {}
                }
            }
        }
    }
    println!("Runner is finished");
}

pub struct Runner {
    receiver: Option<Receiver<ReportMessage>>,
    runner_handle: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn start(config: Config) -> Runner {
        let (report_sender, report_receiver) = channel::<ReportMessage>(100);

        let runner_handle = {
            // let sender = status_sender.clone();
            let config = config.clone();
            // let users_counter = users_counter.clone();
            let handler = tokio::spawn(async move {
                runner(config, report_sender).await;
            });
            handler
        };

        return Runner {
            receiver: Some(report_receiver),
            runner_handle: Some(runner_handle),
        };
    }

    pub fn take_receiver(&mut self) -> Result<Receiver<ReportMessage>, &str> {
        if let Some(receiver) = self.receiver.take() {
            return Ok(receiver);
        }
        return Err("Runner is either not started or receiver already taken");
    }

    pub async fn wait_until_finished(&mut self) {
        if let Some(runner) = self.runner_handle.take() {
            runner.await.unwrap();
        }
    }
}
