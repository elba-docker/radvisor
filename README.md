# ![rAdvisor](https://i.imgur.com/aYdn3MV.png)

> Monitors system resource utilization in [Docker](https://www.docker.com/) containers with **high granularity** and **low overhead**, developed in Rust as a custom tool to help detect and analyze millibottlenecks in containerized online systems. Runs by polling the Docker daemon every 1 second (by default) to get a list of active, running containers. From this list, rAdvisor runs a collection thread every 50ms (by default) to get resource utilization data for each active container using Linux [`cgroups`](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch01). These logs are then written in CSV format at `/var/log/docker/stats` (by default).

## 🖨️ Example Output

##### `/var/log/docker/stats/ee998c6b9..._1585108344647176567.log`

```csv
# Version: 1.0.0
# ID: ee998c6b98236b820dd8b57d161ccad635d3e9eb14841679a997bc03e5442942
# Names: ["/thirsty_cannon"]
# Command: bash -c 'while true; do sleep 2s; done'
# Image: ubuntu
# Status: Up 2 hours
# Labels: {}
# Ports: []
# Created: 2020-03-24 23:04:38 UTC
# Size: None
# Root FS Size: None
# Poll time: 1585108344597589099
# Initialized at: 1585108344647176567
read,pids.current,pids.max,cpu.usage.total,cpu.usage.system,cpu.usage.user,cpu.usage.percpu,cpu.stat.user,cpu.stat.system,cpu.throttling.periods,cpu.throttling.throttled.count,cpu.throttling.throttled.time,memory.usage.current,memory.usage.max,memory.limit.hard,memory.limit.soft,memory.failcnt,memory.hiearchical_limit.memory,memory.hiearchical_limit.memoryswap,memory.cache,memory.rss.all,memory.rss.huge,memory.mapped,memory.swap,memory.paged.in,memory.paged.out,memory.fault.total,memory.fault.major,memory.anon.inactive,memory.anon.active,memory.file.inactive,memory.file.active,memory.unevictable,blkio.service.bytes,blkio.service.ios,blkio.service.time,blkio.queued,blkio.wait,blkio.merged,blkio.time,blkio.sectors
1585108344647439685,2,max,24193732125,0,24193732125,24193732125,584,241,0,0,0,100966400,3927359488,9223372036854771712,9223372036854771712,0,9223372036854771712,,19013632,79917056,0,2273280,,1448094,1423941,2052934,310,79712256,204800,14966784,4046848,0,"8:0 Read 34787328,8:0 Write 74403840,8:0 Sync 37494784,8:0 Async 71696384,8:0 Total 109191168,Total 109191168","8:0 Read 1736,8:0 Write 24054,8:0 Sync 15904,8:0 Async 9886,8:0 Total 25790,Total 25790","8:0 Read 553729709,8:0 Write 467872175,8:0 Sync 569993511,8:0 Async 451608373,8:0 Total 1021601884,Total 1021601884","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 341027463,8:0 Write 183051407147,8:0 Sync 710185876,8:0 Async 182682248734,8:0 Total 183392434610,Total 183392434610","8:0 Read 112,8:0 Write 5989,8:0 Sync 112,8:0 Async 5989,8:0 Total 6101,Total 6101",8:0 1677059003,8:0 213264
1585108344697049284,2,max,24193732125,0,24193732125,24193732125,584,241,0,0,0,100966400,3927359488,9223372036854771712,9223372036854771712,0,9223372036854771712,,19013632,79917056,0,2273280,,1448094,1423941,2052934,310,79712256,204800,14966784,4046848,0,"8:0 Read 34787328,8:0 Write 74403840,8:0 Sync 37494784,8:0 Async 71696384,8:0 Total 109191168,Total 109191168","8:0 Read 1736,8:0 Write 24054,8:0 Sync 15904,8:0 Async 9886,8:0 Total 25790,Total 25790","8:0 Read 553729709,8:0 Write 467872175,8:0 Sync 569993511,8:0 Async 451608373,8:0 Total 1021601884,Total 1021601884","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 341027463,8:0 Write 183051407147,8:0 Sync 710185876,8:0 Async 182682248734,8:0 Total 183392434610,Total 183392434610","8:0 Read 112,8:0 Write 5989,8:0 Sync 112,8:0 Async 5989,8:0 Total 6101,Total 6101",8:0 1677059003,8:0 213264
...
```

More information about what each column represents can be found in the [docs page](https://github.com/elba-kubernetes/radvisor/blob/master/docs/collecting.md).

## 📜 Runtime Options

Many of the specific details of collection can be controlled via the command line interface. At the moment, this includes collection/polling intervals and output directory. To view information on the available CLI options, run `radvisor --help`:

```console
$ radvisor --help
radvisor 1.0.0
Joseph Azevedo and Bhanu Garg
Monitors container resource utilization with high granularity and low overhead

USAGE:
    radvisor [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --directory <directory>      target directory to place log files in ({id}.log)
    -i, --interval <interval>        collection interval between log entries (ms)
    -p, --poll <polling-interval>    interval between requests to docker to get containers (ms)
```

## 🏗️ Building

### 🐋 Using Docker

To build rAdvisor using Docker, run the following command (needs to have `docker` installed and running, and likely needs to be run as root):

```
$ sudo make
```

For the Docker build method, the Rust nightly image is used ([rustlang/rust:nightly](https://hub.docker.com/r/rustlang/rust/)) to run a Docker container with the necessary toolchains pre-installed.

### 💽 Directly From Source

To build rAdvisor from source, Rust **nightly** is used. To install Rust nightly, we recommend using [rustup](https://rustup.rs/) to install the Rust toolchain, and then running the following command to switch to nightly:

```
$ rustup default nightly
```

Now, in the cloned repository root, run `make compile` to generate a release-grade binary at `./target/release/radvisor`. This build process may take up to ten minutes.

```console
$ make compile
cargo build --release --bins \
-Z unstable-options \
--out-dir /home/jazev/dev/radvisor \
--target x86_64-unknown-linux-gnu
   Compiling libc v0.2.68
   Compiling autocfg v1.0.0
   Compiling cfg-if v0.1.10
   ...
   Compiling shiplift v0.6.0
   Compiling radvisor v0.4.0 (/home/jazev/dev/radvisor)
    Finished release [optimized] target(s) in 4m 52s
$ ./radvisor --version
radvisor 1.0.0
```
