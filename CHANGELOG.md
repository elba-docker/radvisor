# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/elba-docker/radvisor/compare/v1.3.1...HEAD)

---

## [1.3.1](https://github.com/elba-docker/radvisor/compare/v1.3.0...v1.3.1) - 2020-11-18

[![v1.3.1](https://img.shields.io/badge/release-v1.3.1-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.3.1)
### Added

- (internal) Remove dependency on Nightly Rust and change toolchain to Stable Rust 1.47
- (internal) Use single-threaded Tokio executors to reduce number of kernel threads used at runtime
- (internal) Clean up Makefile builds and unify with CI
- Remove support for Windows builds (until actual support for Host Compute Platform is added)
- Add support for specifying the Kubernetes config file:
  - `radvisor run kubernetes --kube-config ~/.kube/config`
- Re-enable ZSH completions on the generated Debian package, and add value hints for all relevant CLI options

---

## [1.3.0](https://github.com/elba-docker/radvisor/compare/v1.2.2...v1.3.0) - 2020-10-11

[![v1.3.0](https://img.shields.io/badge/release-v1.3.0-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.3.0)
### Added

- Block I/O stats have been better-parsed to now be useful as simple scalar values.
  - For `blkio.time` and `blkio.sectors`, this is a single scalar (representing the total of all devices), while for all other block I/O columns, each category is now split and aggregated into four separate columns for read total, write total, sync total, and async total (for all devices).
  - For example, the non-scalar column `blkio.service.bytes` is now 4 scalar columns: `blkio.service.bytes.read`, `blkio.service.bytes.write`, `blkio.service.bytes.sync`, and `blkio.service.bytes.async`

---

## [1.2.2](https://github.com/elba-docker/radvisor/compare/v1.2.1...v1.2.2) - 2020-10-10

[![v1.2.2](https://img.shields.io/badge/release-v1.2.2-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.2.2)

### Added

- Two additional groups of block-io stats have been added: `bfq` and `throttle`, and with them, 4 more logfile columns:
  ```
  blkio.throttle.service.bytes,
  blkio.throttle.service.ios,
  blkio.bfq.service.bytes,
  blkio.bfq.service.ios
  ```

### Fixed

- `PolledAt` is now a standard entry on the logfile header. Fixes regression made in v1.1.5 where `PolledAt` no longer appeared on target logfiles created by the `Docker` provider

---

## [1.2.1](https://github.com/elba-docker/radvisor/compare/v1.2.0...v1.2.1) - 2020-09-27

[![v1.2.1](https://img.shields.io/badge/release-v1.2.1-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.2.1)

### Added

- The buffer size is now parameterized by providing a `--buffer` option to the main CLI. This buffer size is allocated on the heap **for each collection target**, and is used to store the CSV records as they are produced.
- (internal) Upgraded to clap version `"3.0.0-beta.2"`, removing 1/2 of the dependencies on Git package versions (sys-info-rs) is still depended on

---

## [1.2.0](https://github.com/elba-docker/radvisor/compare/v1.1.7...v1.2.0) - 2020-09-25

[![v1.2.0](https://img.shields.io/badge/release-v1.2.0-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.2.0)

### Added

- Buffer flush logging by providing a `--flush-log` option to the CLI that can be used to enable logging to an in-memory buffer when the collection log file buffers get flushed (and written). This is to provide a record of when rAdvisor consumes resources like file I/O to ensure it doesn't confound experimental results.

---

## [1.1.7](https://github.com/elba-docker/radvisor/compare/v1.1.6...v1.1.7) - 2020-04-11

[![v1.1.7](https://img.shields.io/badge/release-v1.1.7-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.1.7)

### Added

- Debian packaging is now included with every release (#10)
- Man files compiled from source in the `/man` folder
- (internal) Additional CLI tool crate in `build/`, called `radvisor-toolbox`, used to generate shell completions and compress docs for packaging
- (internal) Enabled additional lint rules in `clippy::nursery` and `clippy::pedantic`, along with ensuring compliance across the codebase

---

## [1.1.6](https://github.com/elba-docker/radvisor/compare/v1.1.5...v1.1.6) - 2020-04-10

[![v1.1.6](https://img.shields.io/badge/release-v1.1.6-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.1.6)

### Fixed

- NUL byte bug where read fields in the final output were right-padded with NUL characters, caused by writing the entire buffer

---

## [1.1.5](https://github.com/elba-docker/radvisor/compare/v1.1.6...v1.1.5) (YANKED) - 2020-04-10

[![v1.1.5](https://img.shields.io/badge/release-v1.1.5-red)](https://github.com/elba-docker/radvisor/releases/tag/v1.1.5)

### Added

- System information to log metadata, including Linux distribution (if available) and CPU/memory information (#6)
- (internal) Event-based thread communication between the polling and collection threads (#8)

### Changed

- Modified the structure of log header metadata to support the migration to event-based thread communication

---

## [1.1.4](https://github.com/elba-docker/radvisor/compare/tail...v1.1.4) - 2020-04-04

[![v1.1.4](https://img.shields.io/badge/release-v1.1.4-2bab64)](https://github.com/elba-docker/radvisor/releases/tag/v1.1.4)

### Added

- Polling for Docker containers to the Docker daemon, retrieving a list of active, running containers to collect statistics for (`radvisor run docker`)
- Polling for Kubernetes pods to the Kubernetes API, determining what the current node the process is running on and collecting pods that are ccurrently running on that node.
- Cgroup-based statistics collection on Linux for collection targets (pods/containers)
- [CSVY](https://csvy.org/) statistics output with YAML metadata headers and CSV data bodies in `/var/log/radvisor/stats`
