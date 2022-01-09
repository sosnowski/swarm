mod config;
mod runner;
mod reporter;
mod user;

#[tokio::main]
async fn main() {
    println!("Start");
    let config = config::Config::new();

    let mut runner = runner::Runner::start(config.clone());

    let mut reporter = reporter::CmdReporter::start(config.clone(), runner.take_receiver().unwrap());

    runner.wait_until_finished().await;
    reporter.wait_until_finished().await;
    println!("End");
}
