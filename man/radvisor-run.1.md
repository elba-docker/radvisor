% RADVISOR(1) Version 1.1.7 | radvisor User Manual

NAME
====

**radvisor run** - runs radvisor to collect statistics for a updating set of targets

SYNOPSIS
========

> radvisor run \[FLAGS\] \[OPTIONS\] \<provider\>

DESCRIPTION
===========

**radvisor run** runs a collection thread that writes resource statistics to
output CSV files using configurable intervals. It has two modes of operation (providers):

1. **docker** - Collects statistics for containers, polling the docker daemon to get a list of active running containers (every 1s by default)
and using their cgroups to read information on their system resource utilization.
*Likely needs to be run as root*.
2. **kubernetes** - Collects statistics for Kubernetes pods, polling the Kubernetes API server to get a list of all active running pods
that have been scheduled on the current machine's node, using the cgroups for each pod.
*Needs to be a part of an active cluster and needs to be able to find the Kubernetes config file*.

ARGS:
-----

\<provider\>

:   Provider to use to generate collection targets (such as
    containers/pods)

FLAGS:
------

**-h**, **\--help**

:   Prints help information

**-q**, **\--quiet**

:   Whether to run in quiet mode (minimal output)

**-v**, **\--verbose**

:   Whether to run in verbose mode (maximum output)

**-V**, **\--version**

:   Prints version information

OPTIONS:
--------

**-c**, **\--color** \<color-mode\>

> Color display mode for stdout/stderr output \[default: auto\]

**-d**, **\--directory** \<directory\>

> Target directory to place log files in ({id}\_{timestamp}.log) \[default: /var/log/radvisor/stats\]

**-i**, **\--interval** \<interval\>

> Collection interval between log entries \[default: 50ms\]

**-p**, **\--poll** \<polling-interval\>

> Interval between requests to providers to get targets \[default: 1000ms\]

**-f**, **\--flush-log** \<flush-log\>

> (optional) Target location to write an buffer flush event log

ENVIRONMENT
===========

**DOCKER_HOST**

:   URL of the docker daemon to use when running the **docker** provider.
    Defaults to `unix:///var/run/docker.sock`

BUGS
====

To report bugs found in rAdvisor, feel free to make a new issue on the GitHub repository:
<https://github.com/elba-kubernetes/radvisor/issues/new>

AUTHOR
======

Joseph Azevedo <https://jazevedo.me>

LICENSE
=======

This project is licensed under the MIT License <https://github.com/elba-kubernetes/radvisor/blob/develop/LICENSE>.
