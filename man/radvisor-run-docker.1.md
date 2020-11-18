% RADVISOR(1) Version 1.3.1 | radvisor User Manual

NAME
====

**radvisor run docker** - runs radvisor to collect statistics for all active containers on the current machine

SYNOPSIS
========

**radvisor run docker** \[FLAGS\] \[OPTIONS\]

DESCRIPTION
===========

**radvisor run docker** runs a collection thread that writes resource statistics to
output CSV files using configurable intervals. While running, it collects statistics for containers by polling the docker daemon to get a list of active running containers (every 1s by default) and using their cgroups to read information on their system resource utilization.

Likely needs to be run as root.

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

:   URL of the docker daemon to use.
    Defaults to `unix:///var/run/docker.sock`

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
**radvisor-run-kubernetes(1)**

LICENSE
=======

This project is licensed under the MIT License <https://github.com/elba-docker/radvisor/blob/develop/LICENSE>.
