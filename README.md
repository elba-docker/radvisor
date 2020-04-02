# ![rAdvisor](https://i.imgur.com/aYdn3MV.png)

> Monitors & collects system resource utilization on Linux for [Docker](https://www.docker.com/) containers and [Kubernetes](https://kubernetes.io/) pods with **high granularity** and **low overhead**, emitting resource utilization logs in [CSVY](https://csvy.org/) (csv + yaml) format. Originally, developed in Rust as a custom tool to help detect and analyze millibottlenecks in containerized online systems, rAdvisor runs by polling the target provider (either the local Docker daemon or the Kubernetes API server) every 1 second to get a list of active, running containers/pods. From this list, rAdvisor runs a collection thread every 50ms to get resource utilization data for each active target using Linux [`cgroups`](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch01), outputting the resultant logs in `/var/log/radvisor/stats`.

## 🖨️ Example Output

> **Note**: filenames correspond to the ID/UID of the container/pod, with the collector initialization timestamp appended at the end.

### 🐋 Docker

##### `/var/log/radvisor/stats/c0cd2077ec95e1b340e85c2...b_1585108344.log`

```yaml
---
Version: 1.1.2
Provider: docker
Created: "2020-03-24T07:27:49Z"
Command: "bash -c 'while true; do sleep 2; done'"
Id: c0cd2077ec95e1b340e85c206e0ffb182ff94dbac16b43a72785fc5e7d0859ab
Image: ubuntu
Labels: {}
Names:
  - /cranky_tereshkova
Ports: []
Status: Up 24 minutes
SizeRw: ~
SizeRootFs: ~
PolledAt: 1585698933605695131
Cgroup: /docker/c0cd2077ec95e1b340e85c206e0ffb182ff94dbac16b43a72785fc5e7d0859ab
CgroupDriver: cgroupfs
InitializedAt: 1585698933654366635
---
read,pids.current,pids.max,cpu.usage.total,cpu.usage.system,cpu.usage.user,cpu.usage.percpu,cpu.stat.user,cpu.stat.system,cpu.throttling.periods,cpu.throttling.throttled.count,cpu.throttling.throttled.time,memory.usage.current,memory.usage.max,memory.limit.hard,memory.limit.soft,memory.failcnt,memory.hierarchical_limit.memory,memory.hierarchical_limit.memoryswap,memory.cache,memory.rss.all,memory.rss.huge,memory.mapped,memory.swap,memory.paged.in,memory.paged.out,memory.fault.total,memory.fault.major,memory.anon.inactive,memory.anon.active,memory.file.inactive,memory.file.active,memory.unevictable,blkio.service.bytes,blkio.service.ios,blkio.service.time,blkio.queued,blkio.wait,blkio.merged,blkio.time,blkio.sectors
1585698933654984250,2,max,24193732125,0,24193732125,24193732125,584,241,0,0,0,100966400,3927359488,9223372036854771712,9223372036854771712,0,9223372036854771712,,19013632,79917056,0,2273280,,1448094,1423941,2052934,310,79712256,204800,14966784,4046848,0,"8:0 Read 34787328,8:0 Write 74403840,8:0 Sync 37494784,8:0 Async 71696384,8:0 Total 109191168,Total 109191168","8:0 Read 1736,8:0 Write 24054,8:0 Sync 15904,8:0 Async 9886,8:0 Total 25790,Total 25790","8:0 Read 553729709,8:0 Write 467872175,8:0 Sync 569993511,8:0 Async 451608373,8:0 Total 1021601884,Total 1021601884","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 341027463,8:0 Write 183051407147,8:0 Sync 710185876,8:0 Async 182682248734,8:0 Total 183392434610,Total 183392434610","8:0 Read 112,8:0 Write 5989,8:0 Sync 112,8:0 Async 5989,8:0 Total 6101,Total 6101",8:0 1677059003,8:0 213264
1585698933705493710,2,max,24193732125,0,24193732125,24193732125,584,241,0,0,0,100966400,3927359488,9223372036854771712,9223372036854771712,0,9223372036854771712,,19013632,79917056,0,2273280,,1448094,1423941,2052934,310,79712256,204800,14966784,4046848,0,"8:0 Read 34787328,8:0 Write 74403840,8:0 Sync 37494784,8:0 Async 71696384,8:0 Total 109191168,Total 109191168","8:0 Read 1736,8:0 Write 24054,8:0 Sync 15904,8:0 Async 9886,8:0 Total 25790,Total 25790","8:0 Read 553729709,8:0 Write 467872175,8:0 Sync 569993511,8:0 Async 451608373,8:0 Total 1021601884,Total 1021601884","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 341027463,8:0 Write 183051407147,8:0 Sync 710185876,8:0 Async 182682248734,8:0 Total 183392434610,Total 183392434610","8:0 Read 112,8:0 Write 5989,8:0 Sync 112,8:0 Async 5989,8:0 Total 6101,Total 6101",8:0 1677059003,8:0 213264
...
```

More information about what each column represents can be found in the [docs page](https://github.com/elba-kubernetes/radvisor/blob/master/docs/collecting.md).

### ⚓ Kubernetes

##### `/var/log/radvisor/stats/9f0b1893-15e7-4...c_1585470948.log.log`

```yaml
---
Version: 1.1.2
Provider: kubernetes
Uid: 9f0b1893-15e7-442a-966a-b0d19a35fc1c
Name: kube-proxy-hsplg
CreatedAt: "2020-03-29T04:32:35Z"
Labels:
  controller-revision-hash: c8bb659c5
  k8s-app: kube-proxy
  pod-template-generation: "1"
Namespace: kube-system
NodeName: node2.radvisor-sandbox.infosphere-pg0.clemson.cloudlab.us
HostIp: 130.127.133.26
Phase: Running
QosClass: BestEffort
StartedAt: "2020-03-29T04:32:36Z"
PolledAt: 1585470948008442929
Cgroup: /kubepods.slice/kubepods-besteffort.slice/kubepods-besteffort-pod9f0b1893_15e7_442a_966a_b0d19a35fc1c.slice
CgroupDriver: systemd
InitializedAt: 1585470948030565581
---
read,pids.current,pids.max,cpu.usage.total,cpu.usage.system,cpu.usage.user,cpu.usage.percpu,cpu.stat.user,cpu.stat.system,cpu.throttling.periods,cpu.throttling.throttled.count,cpu.throttling.throttled.time,memory.usage.current,memory.usage.max,memory.limit.hard,memory.limit.soft,memory.failcnt,memory.hierarchical_limit.memory,memory.hierarchical_limit.memoryswap,memory.cache,memory.rss.all,memory.rss.huge,memory.mapped,memory.swap,memory.paged.in,memory.paged.out,memory.fault.total,memory.fault.major,memory.anon.inactive,memory.anon.active,memory.file.inactive,memory.file.active,memory.unevictable,blkio.service.bytes,blkio.service.ios,blkio.service.time,blkio.queued,blkio.wait,blkio.merged,blkio.time,blkio.sectors
1585470948047082140,17,max,7009726164,0,7009726164,1815447696 1685281115 1688834976 1640529633 173096 513545 762835 0 85256 0 3989326 374099 0 0 350908 307147 0 0 0 0 1093337 450393 1354485 205130 3359333 313783 4052594 0 3541008 0 16083586 1245576 1916663 1761707 46435538 4895736 15666894 40298012 13581147 16821610,330,182,0,0,0,15884288,17481728,9223372036854771712,9223372036854771712,0,270399004672,,372736,9756672,0,0,,39575,37102,96626,0,0,7618560,2412544,98304,0,"8:0 Read 299008,8:0 Write 12288,8:0 Sync 311296,8:0 Async 0,8:0 Total 311296,Total 311296","8:0 Read 15,8:0 Write 3,8:0 Sync 18,8:0 Async 0,8:0 Total 18,Total 18","8:0 Read 82648611,8:0 Write 3070793,8:0 Sync 85719404,8:0 Async 0,8:0 Total 85719404,Total 85719404","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 205655826,8:0 Write 3368384,8:0 Sync 209024210,8:0 Async 0,8:0 Total 209024210,Total 209024210","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0",8:0 97277586,8:0 608
1585470948099117741,17,max,7009726164,0,7009726164,1815447696 1685281115 1688834976 1640529633 173096 513545 762835 0 85256 0 3989326 374099 0 0 350908 307147 0 0 0 0 1093337 450393 1354485 205130 3359333 313783 4052594 0 3541008 0 16083586 1245576 1916663 1761707 46435538 4895736 15666894 40298012 13581147 16821610,330,182,0,0,0,15884288,17481728,9223372036854771712,9223372036854771712,0,270399004672,,372736,9756672,0,0,,39575,37102,96626,0,0,7618560,2412544,98304,0,"8:0 Read 299008,8:0 Write 12288,8:0 Sync 311296,8:0 Async 0,8:0 Total 311296,Total 311296","8:0 Read 15,8:0 Write 3,8:0 Sync 18,8:0 Async 0,8:0 Total 18,Total 18","8:0 Read 82648611,8:0 Write 3070793,8:0 Sync 85719404,8:0 Async 0,8:0 Total 85719404,Total 85719404","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0","8:0 Read 205655826,8:0 Write 3368384,8:0 Sync 209024210,8:0 Async 0,8:0 Total 209024210,Total 209024210","8:0 Read 0,8:0 Write 0,8:0 Sync 0,8:0 Async 0,8:0 Total 0,Total 0",8:0 97277586,8:0 608
...
```

## 📜 Runtime Options

Many of the specific details of collection can be controlled via the command line interface. At the moment, this includes collection/polling intervals and output directory. To view information on the available CLI options, run `radvisor --help`:

```console
$ radvisor --help
radvisor 1.1.2
Joseph Azevedo and Bhanu Garg
Monitors container resource utilization with high granularity and low overhead

USAGE:
    radvisor [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --directory <directory>      target directory to place log files in ({id}_{timestamp}.log) [default: /var/log/radvisor/stats]
    -i, --interval <interval>        collection interval between log entries [default: 50ms]
    -p, --poll <polling-interval>    interval between requests to providers to get targets [default: 1000ms]

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    run     runs a collection thread that writes resource statistics to output CSV files
```

### 🧮 Subcommands

#### `radvisor run`

```console
$ radvisor run <provider>
```

The main subcommand of rAdvisor is `run`, which additionally requires the target provider (Docker or Kubernetes) to use to discover collection targets. For example, to run rAdvisor and collect resource utilization statistics on Docker containers each 40ms, the following command would be used:

```console
$ radvisor run docker -i 40ms
Initializing Docker API provider
Beginning statistics collection
Identified cgroupfs as cgroup driver
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
   Compiling radvisor v1.1.2 (/home/jazev/dev/radvisor)
    Finished release [optimized] target(s) in 4m 52s
$ ./radvisor --version
radvisor 1.1.2
```
