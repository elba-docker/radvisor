use std::vec::Vec;
use std::sync::mpsc::Sender;

use shiplift::Docker;
use shiplift::rep::Container;
use tokio::runtime::Runtime;
use eventual::Timer;

/// Thread function that updates the container list each second by default
pub fn run(tx: Sender<Vec<String>>, interval: u32) -> () {
    let docker = Docker::new();
    let timer = Timer::new();
    let ticks = timer.interval_ms(interval).iter();
    for _ in ticks {
        let containers: Vec<Container> = match get_containers(&docker) {
            Ok(containers) => containers,
            Err(_)         => Vec::with_capacity(0)
        };
        // Reduce container vector to list of ids
        let to_collect: Vec<String> = containers.into_iter()
            .filter_map(|c| match should_collect_stats(&c) {
                true  => Some(c.id),
                false => None
            })
            .collect::<Vec<_>>();
        // If sending fails, then panic the thread anyways
        tx.send(to_collect).unwrap();
    }
}

/// Determines whether the monitoring process can connect to the dockerd socket
pub fn can_connect() -> bool {
    let docker = Docker::new();
    let future = docker.ping();
    // Block on the future and match on the result
    match Runtime::new().unwrap().block_on(future) {
        Ok(_)  => true,
        Err(_) => false
    }
}

/// Gets all containers running locally on the docker daemon via shiplift
fn get_containers(docker: &Docker) -> Result<Vec<Container>, shiplift::errors::Error> {
    let future = docker
        .containers()
        .list(&Default::default());
    // Block on the future and return the result
    Runtime::new().unwrap().block_on(future)
}

/// Whether radvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool {
    // TODO implement
    true
}
