# ![rAdvisor](https://i.imgur.com/aYdn3MV.png)
![build/test](https://github.com/elba-docker/radvisor/workflows/build/test/badge.svg?branch=master) ![security](https://github.com/elba-docker/radvisor/workflows/security/badge.svg?branch=master) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](/LICENSE) [![Latest release](https://img.shields.io/github/v/release/elba-docker/radvisor?color=2bab64)](https://github.com/elba-docker/radvisor/releases) [![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Felba-docker%2Fradvisor.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Felba-docker%2Fradvisor?ref=badge_shield)

> Monitors & collects system resource utilization on Linux for [Docker](https://www.docker.com/) containers and [Kubernetes](https://kubernetes.io/) pods with **fine granularity** and **low overhead**, emitting resource utilization logs in [CSVY](https://csvy.org/) (csv + yaml) format. Originally, developed in Rust as a custom tool to help detect and analyze millibottlenecks in containerized online systems, rAdvisor runs by polling the target provider (either the local Docker daemon or the Kubernetes API server) every 1 second to get a list of active, running containers/pods. From this list, rAdvisor runs a collection thread every 50ms to get resource utilization data for each active target using Linux [`cgroups`](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch01), outputting the resultant logs in `/var/log/radvisor/stats`.

## 🖨️ Example Output

> **Note**: filenames correspond to the ID/UID of the container/pod, with the collector initialization timestamp appended at the end.

### 🐋 Docker

##### `/var/log/radvisor/stats/c0cd2077ec95e1b340e85c2...b_1585108344.log`

```yaml
---
Version: 1.3.1
Provider: docker
Metadata:
  Created: "2020-10-11T04:22:18Z"
  Command: "bash -c 'sleep 2s; apt-get update; sleep 2s; DEBIAN_FRONTEND=noninteractive apt-get install -y stress wget; sleep 2s; dd if=/dev/zero of=/tmp/file1 bs=512M count=1 oflag=direct; sleep 2s; stress --cpu 8 --io 4 --vm 4 --vm-bytes 1024M --timeout 10s; sleep 2s; wget \"http://ipv4.download.thinkbroadband.com/10MB.zip\"; sleep 2s'"
  Id: 7762ff15c99a2d238f4d26c22b5eda5b97ebc03bd0a711693104dcb6f71fe411
  Image: ubuntu
  Labels: {}
  Names:
    - /silly_elion
  Ports: []
  Status: Up Less than a second
  SizeRw: ~
  SizeRootFs: ~
PerfTable:
  Delimiter: ","
  Columns:
    cpu.usage.percpu:
      Type: int
      Count: 32
    read:
      Type: epoch19
System:
  OsType: Linux
  OsRelease: 4.15.0
  Distribution:
    Id: ubuntu
    IdLike: debian
    Name: Ubuntu
    PrettyName: Ubuntu 18.04.1 LTS
    Version: 18.04.1 LTS (Bionic Beaver)
    VersionId: "18.04"
    VersionCodename: bionic
    CpeName: ~
    BuildId: ~
    Variant: ~
    VariantId: ~
  MemoryTotal: 65870408
  SwapTotal: 3145724
  Hostname: node-0.sandbox.infosphere.emulab.net
  CpuCount: 32
  CpuOnlineCount: 4
  CpuSpeed: 1279
Cgroup: system.slice/docker-7762ff15c99a2d238f4d26c22b5eda5b97ebc03bd0a711693104dcb6f71fe411.scope
CgroupDriver: systemd
PolledAt: 1602390140142271945
InitializedAt: 1602390140157676566
---
read,pids.current,pids.max,cpu.usage.total,cpu.usage.system,cpu.usage.user,cpu.usage.percpu,cpu.stat.user,cpu.stat.system,cpu.throttling.periods,cpu.throttling.throttled.count,cpu.throttling.throttled.time,memory.usage.current,memory.usage.max,memory.limit.hard,memory.limit.soft,memory.failcnt,memory.hierarchical_limit.memory,memory.hierarchical_limit.memoryswap,memory.cache,memory.rss.all,memory.rss.huge,memory.mapped,memory.swap,memory.paged.in,memory.paged.out,memory.fault.total,memory.fault.major,memory.anon.inactive,memory.anon.active,memory.file.inactive,memory.file.active,memory.unevictable,blkio.time,blkio.sectors,blkio.service.bytes.read,blkio.service.bytes.write,blkio.service.bytes.sync,blkio.service.bytes.async,blkio.service.ios.read,blkio.service.ios.write,blkio.service.ios.sync,blkio.service.ios.async,blkio.service.time.read,blkio.service.time.write,blkio.service.time.sync,blkio.service.time.async,blkio.queued.read,blkio.queued.write,blkio.queued.sync,blkio.queued.async,blkio.wait.read,blkio.wait.write,blkio.wait.sync,blkio.wait.async,blkio.merged.read,blkio.merged.write,blkio.merged.sync,blkio.merged.async,blkio.throttle.service.bytes.read,blkio.throttle.service.bytes.write,blkio.throttle.service.bytes.sync,blkio.throttle.service.bytes.async,blkio.throttle.service.ios.read,blkio.throttle.service.ios.write,blkio.throttle.service.ios.sync,blkio.throttle.service.ios.async,blkio.bfq.service.bytes.read,blkio.bfq.service.bytes.write,blkio.bfq.service.bytes.sync,blkio.bfq.service.bytes.async,blkio.bfq.service.ios.read,blkio.bfq.service.ios.write,blkio.bfq.service.ios.sync,blkio.bfq.service.ios.async
1602390175053135973,18,4915,45675783181,0,45675783181,9719044209 12310201631 11027849186 12618688155 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0,2925,1668,0,0,0,2802823168,3667771392,9223372036854771712,9223372036854771712,0,9223372036854771712,,35323904,2754781184,0,28672,,6837817,6156636,6854722,0,0,2754711552,7380992,27942912,0,2273087370,1306336,0,668844032,662777856,6066176,0,331937,331753,184,0,68057100860,68011971780,45129080,0,0,0,0,0,222907407415,222860666999,46740416,0,32,0,32,0,668844032,662777856,6066176,0,331937,331753,184,,,,,,,,
1602390175103189646,18,4915,45876609757,0,45876610443,9767491855 12362201213 11076227542 12670689833 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0,2938,1676,0,0,0,2968367104,3667771392,9223372036854771712,9223372036854771712,0,9223372036854771712,,35323904,2920067072,0,28672,,6878171,6156636,6895076,0,0,2919976960,7380992,27942912,0,2273087370,1306336,0,668844032,662777856,6066176,0,333749,333565,184,0,68057100860,68011971780,45129080,0,0,0,0,0,222907407415,222860666999,46740416,0,32,0,32,0,668844032,662777856,6066176,0,333750,333566,184,,,,,,,,
...
```

More information about what each column represents can be found in the [docs page](https://github.com/elba-docker/radvisor/blob/master/docs/collecting.md).

### ⚓ Kubernetes

##### `/var/log/radvisor/stats/9f0b1893-15e7-4...c_1585470948.log.log`

```yaml
---
Version: 1.3.1
Provider: kubernetes
Metadata:
  Uid: 9f0b1893-15e7-442a-966a-b0d19a35fc1c
  Name: kube-proxy-hsplg
  CreatedAt: "2020-03-29T04:32:35Z"
  Labels:
    controller-revision-hash: c8bb659c5
    k8s-app: kube-proxy
    pod-template-generation: "1"
  Namespace: kube-system
  NodeName: node-0.sandbox.infosphere.emulab.net
  HostIp: 130.127.133.26
  Phase: Running
  QosClass: BestEffort
  StartedAt: "2020-03-29T04:32:36Z"
PerfTable:
  Delimiter: ","
  Columns:
    cpu.usage.percpu:
      Type: int
      Count: 32
    read:
      Type: epoch19
System:
  OsType: Linux
  OsRelease: 4.15.0
  Distribution:
    Id: ubuntu
    IdLike: debian
    Name: Ubuntu
    PrettyName: Ubuntu 18.04.1 LTS
    Version: 18.04.1 LTS (Bionic Beaver)
    VersionId: "18.04"
    VersionCodename: bionic
    CpeName: ~
    BuildId: ~
    Variant: ~
    VariantId: ~
  MemoryTotal: 65870408
  SwapTotal: 3145724
  Hostname: node-0.sandbox.infosphere.emulab.net
  CpuCount: 32
  CpuOnlineCount: 32
  CpuSpeed: 1198
PolledAt: 1585470948008442929
Cgroup: /kubepods.slice/kubepods-besteffort.slice/kubepods-besteffort-pod9f0b1893_15e7_442a_966a_b0d19a35fc1c.slice
CgroupDriver: systemd
InitializedAt: 1585470948030565581
---
read,pids.current,pids.max,cpu.usage.total,cpu.usage.system,cpu.usage.user,cpu.usage.percpu,cpu.stat.user,cpu.stat.system,cpu.throttling.periods,cpu.throttling.throttled.count,cpu.throttling.throttled.time,memory.usage.current,memory.usage.max,memory.limit.hard,memory.limit.soft,memory.failcnt,memory.hierarchical_limit.memory,memory.hierarchical_limit.memoryswap,memory.cache,memory.rss.all,memory.rss.huge,memory.mapped,memory.swap,memory.paged.in,memory.paged.out,memory.fault.total,memory.fault.major,memory.anon.inactive,memory.anon.active,memory.file.inactive,memory.file.active,memory.unevictable,blkio.time,blkio.sectors,blkio.service.bytes.read,blkio.service.bytes.write,blkio.service.bytes.sync,blkio.service.bytes.async,blkio.service.ios.read,blkio.service.ios.write,blkio.service.ios.sync,blkio.service.ios.async,blkio.service.time.read,blkio.service.time.write,blkio.service.time.sync,blkio.service.time.async,blkio.queued.read,blkio.queued.write,blkio.queued.sync,blkio.queued.async,blkio.wait.read,blkio.wait.write,blkio.wait.sync,blkio.wait.async,blkio.merged.read,blkio.merged.write,blkio.merged.sync,blkio.merged.async,blkio.throttle.service.bytes.read,blkio.throttle.service.bytes.write,blkio.throttle.service.bytes.sync,blkio.throttle.service.bytes.async,blkio.throttle.service.ios.read,blkio.throttle.service.ios.write,blkio.throttle.service.ios.sync,blkio.throttle.service.ios.async,blkio.bfq.service.bytes.read,blkio.bfq.service.bytes.write,blkio.bfq.service.bytes.sync,blkio.bfq.service.bytes.async,blkio.bfq.service.ios.read,blkio.bfq.service.ios.write,blkio.bfq.service.ios.sync,blkio.bfq.service.ios.async
1602390175053135973,18,4915,45675783181,0,45675783181,9719044209 12310201631 11027849186 12618688155 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0,2925,1668,0,0,0,2802823168,3667771392,9223372036854771712,9223372036854771712,0,9223372036854771712,,35323904,2754781184,0,28672,,6837817,6156636,6854722,0,0,2754711552,7380992,27942912,0,2273087370,1306336,0,668844032,662777856,6066176,0,331937,331753,184,0,68057100860,68011971780,45129080,0,0,0,0,0,222907407415,222860666999,46740416,0,32,0,32,0,668844032,662777856,6066176,0,331937,331753,184,,,,,,,,
1602390175103189646,18,4915,45876609757,0,45876610443,9767491855 12362201213 11076227542 12670689833 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0,2938,1676,0,0,0,2968367104,3667771392,9223372036854771712,9223372036854771712,0,9223372036854771712,,35323904,2920067072,0,28672,,6878171,6156636,6895076,0,0,2919976960,7380992,27942912,0,2273087370,1306336,0,668844032,662777856,6066176,0,333749,333565,184,0,68057100860,68011971780,45129080,0,0,0,0,0,222907407415,222860666999,46740416,0,32,0,32,0,668844032,662777856,6066176,0,333750,333566,184,,,,,,,,
...
```

## 📜 Runtime Options

Many of the specific details of collection can be controlled via the command line interface. At the moment, this includes collection/polling intervals and output directory. To view information on the available CLI options, run `radvisor help`:

```console
$ radvisor help
radvisor 1.3.1
Joseph Azevedo <joseph.az@gatech.edu>, Bhanu Garg <bgarg6@gatech.edu>
Monitors container resource utilization with high granularity and low overhead

USAGE:
    radvisor [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      Whether to run in quiet mode (minimal output)
    -v, --verbose    Whether to run in verbose mode (maximum output)
    -V, --version    Prints version information

OPTIONS:
    -c, --color <color-mode>    Color display mode for stdout/stderr output [default: auto]

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    run     Runs a collection thread that writes resource statistics to output CSV files
```

### 📇 Subcommands

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

### ☑️ Supported Operating Systems

At the moment, rAdvisor only supports Linux (due to its heavy reliance on cgroups), though there is a tracking issue for extending its functionality to work with Window's own first-party containerization API, HCS: [radvisor/issues/#3](https://github.com/elba-docker/radvisor/issues/3).

## 🏗️ Building

### 🐋 Using Docker

To build rAdvisor using Docker, run the following command (needs to have `docker` installed and running, and likely needs to be run as root):

```
$ sudo make
```

For the Docker build method, the Rust stable image is used ([rust](https://hub.docker.com/_/rust)) to run a Docker container with the necessary toolchains pre-installed.

### 💽 Directly From Source

To build rAdvisor from source, Rust **stable** is used. We recommend using [rustup](https://rustup.rs/) to install the Rust toolchain.

Now, in the cloned repository root, run `make compile` to generate a release-grade binary at `./target/release/radvisor`. This build process may take up to ten minutes.

```console
$ make compile
cargo build --release --bins \
--target x86_64-unknown-linux-gnu
   Compiling libc v0.2.68
   Compiling autocfg v1.0.0
   Compiling cfg-if v0.1.10
   ...
   Compiling shiplift v0.6.0
   Compiling radvisor v1.3.1 (/home/jazev/dev/radvisor)
    Finished release [optimized] target(s) in 4m 52s
$ ./radvisor --version
radvisor 1.3.1
```

## ⚖️ License

This project is licensed under the [MIT license](/LICENSE).

### 🔍 FOSSA Status

[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Felba-docker%2Fradvisor.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Felba-docker%2Fradvisor?ref=badge_large)
