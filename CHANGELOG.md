# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/elba-kubernetes/radvisor/compare/v1.1.7...HEAD)

---

## [1.1.7](https://github.com/elba-kubernetes/radvisor/compare/v1.1.6...v1.1.7) - 2020-04-11

[![v1.1.7](https://img.shields.io/badge/release-v1.1.7-2bab64)](https://github.com/elba-kubernetes/radvisor/releases/tag/v1.1.7)

### Added

- Debian packaging is now included with every release (#10)
- Man files compiled from source in the `/man` folder
- (internal) Additional CLI tool crate in `build/`, called `radvisor-toolbox`, used to generate shell completions and compress docs for packaging
- (internal) Enabled additional lint rules in `clippy::nursery` and `clippy::pedantic`, along with ensuring compliance across the codebase

---

## [1.1.6](https://github.com/elba-kubernetes/radvisor/compare/v1.1.5...v1.1.6) - 2020-04-10

[![v1.1.6](https://img.shields.io/badge/release-v1.1.6-2bab64)](https://github.com/elba-kubernetes/radvisor/releases/tag/v1.1.6)

### Fixed

- NUL byte bug where read fields in the final output were right-padded with NUL characters, caused by writing the entire buffer

---

## [1.1.5](https://github.com/elba-kubernetes/radvisor/compare/v1.1.6...v1.1.5) (YANKED) - 2020-04-10

[![v1.1.5](https://img.shields.io/badge/release-v1.1.5-red)](https://github.com/elba-kubernetes/radvisor/releases/tag/v1.1.5)

### Added

- System information to log metadata, including Linux distribution (if available) and CPU/memory information (#6)
- (internal) Event-based thread communication between the polling and collection threads (#8)

### Changed

- Modified the structure of log header metadata to support the migration to event-based thread communication

---

## [1.1.4](https://github.com/elba-kubernetes/radvisor/compare/tail...v1.1.4) - 2020-04-04

[![v1.1.4](https://img.shields.io/badge/release-v1.1.4-2bab64)](https://github.com/elba-kubernetes/radvisor/releases/tag/v1.1.4)

### Added

- Polling for Docker containers to the Docker daemon, retrieving a list of active, running containers to collect statistics for (`radvisor run docker`)
- Polling for Kubernetes pods to the Kubernetes API, determining what the current node the process is running on and collecting pods that are ccurrently running on that node.
- Cgroup-based statistics collection on Linux for collection targets (pods/containers)
- [CSVY](https://csvy.org/) statistics output with YAML metadata headers and CSV data bodies in `/var/log/radvisor/stats`
