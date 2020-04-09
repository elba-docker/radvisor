# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased](https://github.com/elba-kubernetes/radvisor/compare/v1.1.4...HEAD)

### [1.1.4](https://github.com/elba-kubernetes/radvisor/compare/tail...v1.1.4) - 2020-04-04

[![v1.1.4](https://img.shields.io/badge/release-v1.1.4-blueviolet)](https://github.com/elba-kubernetes/radvisor/releases/tag/v1.1.4)

### Added

- Polling for Docker containers to the Docker daemon, retrieving a list of active, running containers to collect statistics for (`radvisor run docker`)
- Polling for Kubernetes pods to the Kubernetes API, determining what the current node the process is running on and collecting pods that are ccurrently running on that node.
- Cgroup-based statistics collection on Linux for collection targets (pods/containers)
- [CSVY](https://csvy.org/) statistics output with YAML metadata headers and CSV data bodies in `/var/log/radvisor/stats`
