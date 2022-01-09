
use std::ops::Not;
use std::sync::{Arc, Mutex};
use std::future::Future;
use futures::future::join_all;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::{Client, Uri};

use tokio::sync::mpsc::{ Receiver, channel, Sender};
use tokio::task::{JoinHandle, JoinError};
use tokio::time::{sleep, Duration, Instant};

 
use super::message::TaskResult;
use super::{ RunnerMessage};
use crate::{config::Config, scheduler::SchedulerMessage};


async fn execute_tasks(count: u32, config: Arc<Config>, _done_sender: Sender<bool>) -> Vec<TaskResult> {
    let mut i = 0;
    let mut tasks = vec![];

    let https = HttpsConnector::new();
    let http_client = Client::builder().build::<_, hyper::Body>(https);

    while i < count {
        let config = config.clone();
        let task = tokio::spawn(async move {
            let started = Instant::now();
            let response = http_client.get(config.url.parse().unwrap()).await.unwrap();
            let elapsed = started.elapsed();
            let result = TaskResult {
                success: response.status().is_success(),
                error: !response.status().is_success(),
                error_type: "".to_string(), //todo
                duration: elapsed.as_millis(),
            };
            return result;
        });

        tasks.push(task);
        
        i += 1;
    }

    let results = join_all(tasks).await.into_iter().map(|result| {
        return result.unwrap();
    }).collect();

    return results;
}

async fn execute_runner(mut schedule_receiver: Receiver<SchedulerMessage>, runner_sender: Sender<RunnerMessage>, config: Config) {
    let (done_sender, mut done_recv) = channel::<bool>(1);
    let config  = Arc::new(config);

    println!("Runner is running");
    while let Some(msg) = schedule_receiver.recv().await {
        match msg {
            SchedulerMessage::Task { count } => {
                let done_sender = done_sender.clone();
                let runner_sender = runner_sender.clone();
                let config = config.clone();
                tokio::spawn(async move {
                    let time = Instant::now();
                    let results = execute_tasks(count, config, done_sender).await;
                    println!("Tasks execution time: {}", time.elapsed().as_millis());
                    runner_sender.send(RunnerMessage::TaskDone {
                        results,
                    }).await.unwrap();
                });
            },
            SchedulerMessage::Stop {} => break,
        }
    }
    drop(done_sender);
    println!("Runner has stopped, waiting for tasks to finish");
    done_recv.recv().await;
    runner_sender.send(RunnerMessage::Finished {}).await.unwrap();
    println!("Runner finished");
}
pub struct AsyncRunner {
    config: Config,
    task: Option<JoinHandle<()>>,
}

impl AsyncRunner {
    pub fn new(config: Config) -> AsyncRunner {
        return AsyncRunner {
            config,
            task: None,
        };
    }

    pub async fn init(&mut self, schedule_receiver: Receiver<SchedulerMessage>) -> Receiver<RunnerMessage> {
        let config = self.config.clone();
        let (runner_sender, runner_receiver) = channel::<RunnerMessage>(100);
        let task = tokio::spawn(async move {
            execute_runner(schedule_receiver, runner_sender, config).await;
        }); 
        self.task = Some(task);

        return runner_receiver;
    }

    pub async fn wait_for_finished(&mut self) -> Result<(), JoinError> {
        let task = self.task.take();
        return task.unwrap().await;
    }
}
