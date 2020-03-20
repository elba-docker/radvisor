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
        let to_collect: Vec<String> = containers.iter()
            .filter(|&c| should_collect_stats(c))
            .map(|c| c.id.clone())
            .collect::<Vec<_>>();
        // If sending fails, then panic the thread anyways
        tx.send(to_collect).unwrap();
    }
}

/// Gets all containers running locally on the docker daemon via shiplift
fn get_containers(docker: &Docker) -> Result<Vec<Container>, shiplift::errors::Error> {
    let future = docker
        .containers()
        .list(&Default::default());
    Runtime::new().unwrap().block_on(future)
}

/// Whether dadvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool {
    true
}
