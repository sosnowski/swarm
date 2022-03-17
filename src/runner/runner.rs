use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{Duration, Instant, interval};
use tokio::task::JoinHandle;

use crate::config::{Config};
use crate::runner::{UserResult, Scheduler};
use crate::user::http_user;
use super::{ReportMessage, UserStatus};

fn spawn_users(config: &Config, mut users_to_add: usize, status_sender: &Sender<UserStatus>) -> () {

    if users_to_add == 0 {
        return;
    }

    if users_to_add > 20 {
        users_to_add = 20;
    }

    let mut i = 0;
    while i < users_to_add {
        let schedule = config.schedule.clone();
        let status_sender = status_sender.clone();
        tokio::spawn(async move {
            status_sender.send(UserStatus::Created).await.unwrap();
            let user_result = http_user(schedule).await;
            status_sender.send(UserStatus::Finished(user_result)).await.unwrap();
        });
        i += 1;
    }
}

async fn runner(config: Config, report_sender: Sender<ReportMessage>) -> () {
    let (status_sender, mut status_receiver) = channel::<UserStatus>(1000);
    // let (done_sender, mut done_receiver) = channel::<bool>(1);

    let started_at = Instant::now();
    let mut interval = interval(Duration::from_millis(200));

    let mut users_counter: usize = 0;
    let mut queued_results: Vec<UserResult> = vec![];

    let mut scheduler = Scheduler::new(config.workload.clone());

    loop {
        tokio::select! {
            _ = interval.tick() => {

                // send aggregated results
                report_sender.send(ReportMessage {
                    current_users: users_counter,
                    results: queued_results.clone(),
                    duration: started_at.elapsed().as_secs().try_into().unwrap(),
                }).await.unwrap();

                queued_results.clear();

                let target_num_users = scheduler.next();

                if let Some(target_num_users) = target_num_users {
                    spawn_users(&config, target_num_users - users_counter, &status_sender);
                } else {
                    if users_counter == 0 {
                        //wait till all users finish
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
                                users_counter -= 1;
                                queued_results.push(result);
                            },
                        }
                    },
                    None => {}
                }
            },
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
