% RADVISOR(1) Version 1.4.0 | radvisor User Manual

NAME
====

**radvisor run** - runs radvisor to collect statistics for a updating set of targets

SYNOPSIS
========

**radvisor run** \[FLAGS\] \[OPTIONS\] *\<provider\>*

DESCRIPTION
===========

**radvisor run** runs a collection thread that writes resource statistics to
output CSV files using configurable intervals. It has two modes of operation (*providers*) as subcommands:

1. **docker** - Collects statistics for containers, polling the docker daemon to get a list of active running containers (every 1s by default)
and using their cgroups to read information on their system resource utilization.

  Likely needs to be run as root.
2. **kubernetes** - Collects statistics for Kubernetes pods, polling the Kubernetes API server to get a list of all active running pods
that have been scheduled on the current machine's node, using the cgroup for each pod.

  Needs to be a part of an active cluster and needs to be able to find the Kubernetes config file.

SUBCOMMANDS:
------------

docker

:   Runs collection using Docker as the backing target *provider*

kubernetes

:   Runs collection using Kubernetes as the backing target *provider*

help

:   Prints this message or the help of the given subcommand(s)

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

BUGS
====

To report bugs found in rAdvisor, feel free to make a new issue on the GitHub repository:
<https://github.com/elba-docker/radvisor/issues/new>

AUTHOR
======

Joseph Azevedo <https://jazevedo.me>

SEE ALSO
========

**radvisor-run-docker(1)**
**radvisor-run-kubernetes(1)**

LICENSE
=======

This project is licensed under the GNU General Public License v3.0 <https://github.com/elba-docker/radvisor/blob/develop/LICENSE>.
