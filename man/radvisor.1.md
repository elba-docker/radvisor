% RADVISOR(1) Version 1.1.7 | radvisor User Manual

NAME
====

**radvisor** - system resource utilization monitor for containers

SYNOPSIS
========

| **radvisor** \[FLAGS\] \[OPTIONS\] \<SUBCOMMAND\>

DESCRIPTION
===========

**rAdvisor** is a command-line tool that monitors & collects system resource utilization on Linux
for [Docker](https://www.docker.com/) containers and [Kubernetes](https://kubernetes.io/) pods
with **fine granularity** and **low overhead**,
emitting resource utilization logs in [CSVY](https://csvy.org/) (csv + yaml) format.
Originally developed in Rust as a custom tool to help detect and analyze millibottlenecks in containerized online systems,
rAdvisor runs by polling the target provider (either the local Docker daemon or the Kubernetes API server)
every 1 second to get a list of active, running containers/pods.
From this list, rAdvisor runs a collection thread every 50ms to get resource utilization data for each active target
using Linux [`cgroups`](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch01),
outputting the resultant logs in `/var/log/radvisor/stats`.

The primary command is `radvisor run`, which has its own man page at **radvisor-run(1)**.

FLAGS:
------

**-h**, **\--help**

:   Prints help information

**-q**, **\--quiet**

:   whether to run in quiet mode (minimal output)

**-v**, **\--verbose**

:   whether to run in verbose mode (maximum output)

**-V**, **\--version**

:   Prints version information

OPTIONS:
--------

**-c**, **\--color** \<color-mode\>

:   color display mode for stdout/stderr output \[default: auto\]

SUBCOMMANDS:
------------

help

:   Prints this message or the help of the given subcommand(s)

run

:   runs a collection thread that writes resource statistics to output CSV files

BUGS
====

To report bugs found in rAdvisor, feel free to make a new issue on the GitHub repository:
<https://github.com/elba-kubernetes/radvisor/issues/new>

AUTHOR
======

Joseph Azevedo <https://jazevedo.me>

SEE ALSO
========

**radvisor-run(1)**

LICENSE
=======

This project is licensed under the MIT License <https://github.com/elba-kubernetes/radvisor/blob/develop/LICENSE>.
