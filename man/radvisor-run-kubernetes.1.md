% RADVISOR(1) Version 1.3.1 | radvisor User Manual

NAME
====

**radvisor run kubernetes** - runs radvisor to collect statistics for all active pods running on the current host

SYNOPSIS
========

**radvisor run kubernetes** \[FLAGS\] \[OPTIONS\]

DESCRIPTION
===========

**radvisor run kubernetes** runs a collection thread that writes resource statistics to
output CSV files using configurable intervals. While running, it collects statistics for Kubernetes pods, polling the Kubernetes API server to get a list of all active running pods that have been scheduled on the current machine's node, using the cgroups for each pod.

Needs to be a part of an active cluster and needs to be able to find the Kubernetes config file (or specified using **\--kube-config**).

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

**-k**, **\--kube-config** \<path\>

> (optional) Path to load the Kubernetes config from that is used to connect to the cluster. If not given, then radvisor attempts to automatically detect cluster configuration

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

**radvisor-run(1)**
**radvisor-run-docker(1)**

LICENSE
=======

This project is licensed under the MIT License <https://github.com/elba-docker/radvisor/blob/develop/LICENSE>.
