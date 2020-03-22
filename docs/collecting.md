# Runtime Statistics Collection

Runtime statistics for each container are taken from the virtual files for each container's cgroup, located at `/sys/fs/cgroup/<subsystem>/docker/<container id>/file`.

## Subsystems

Each statistic is taken from one of a subset of the cgroup-aware subsystems that run in the Linux kernel. Specifically, statistics are drawn for:

- PIDs
- CPU
- Memory
- Block I/O

### PIDs

The `pids` subsystem contains information about the number of processes running in the container/cgroup.

#### `pids.current`

Maps to `pids.current` in the logs; represents the current number of processes running.

##### ex. `/sys/fs/cgroup/pids/docker/.../pids.current`

```
2
```

#### `pids.max`

Maps to `pids.max` in the logs; represents the maximum number of processes that can possibly run.

##### ex. `/sys/fs/cgroup/pids/docker/.../pids.max`

```
0
```

### CPU

The `cpuacct` and `cpu` subsystems contain information on the CPU runtime costs of each container.

#### `cpuacct.usage`

reports the total CPU time (in nanoseconds) consumed by all tasks in this cgroup (including tasks lower in the hierarchy). Maps to `cpu.usage.total`

##### ex. `/sys/fs/cgroup/cpuacct/docker/.../cpuacct.usage`

```
92159618774
```

#### `cpuacct.usage_sys`

reports the CPU time (in nanoseconds) consumed by all tasks in this cgroup (including tasks lower in the hierarchy) that was spent in system (kernel) mode. Maps to `cpu.usage.system`

##### ex. `/sys/fs/cgroup/cpuacct/docker/.../cpuacct.usage_sys`

```
0
```

#### `cpuacct.usage_user`

reports the CPU time (in nanoseconds) consumed by all tasks in this cgroup (including tasks lower in the hierarchy) that was spent in user mode. Maps to `cpu.usage.user`

##### ex. `/sys/fs/cgroup/cpuacct/docker/.../cpuacct.usage_user`

```
92158508710
```

#### `cpuacct.usage_percpu`

reports the CPU time (in nanoseconds) consumed on each CPU by all tasks in this cgroup (including tasks lower in the hierarchy). Maps to `cpu.usage.percpu`

> **Note**: there are two additional files, `cpuacct.usage_percpu_sys` and `cpuacct.usage_percpu_user` that further break this down into user and kernel modes. However, these have been omitted as they are not needed for the target workload.


##### ex. `/sys/fs/cgroup/cpuacct/docker/.../cpuacct.usage_percpu`

```
10988262282 10955397365 11420884004 12532674907 11310602969 12382279847 12193108713 10432778271 
```

#### `cpuacct.stat`

reports the user and system CPU time consumed by all tasks in this cgroup (including tasks lower in the hierarchy) in the following way:

- `user` — CPU time consumed by tasks in user mode. Maps to `cpu.stat.user`
- `system` — CPU time consumed by tasks in system (kernel) mode. Maps to `cpu.stat.system`

CPU time is reported in the units defined by the `USER_HZ` variable.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-cpuacct)

##### ex. `/sys/fs/cgroup/cpuacct/docker/.../cpuacct.stat`

```
user 2098
system 6839
```

#### `cpu.stat`

Reports CPU time statistics using the following values:

- `nr_periods` — number of period intervals (as specified in cpu.cfs_period_us) that have elapsed. Maps to `cpu.throttling.periods`
- `nr_throttled` — number of times tasks in a cgroup have been throttled (that is, not allowed to run because they have exhausted all of the available time as specified by their quota). Maps to `cpu.throttling.throttled`
- `throttled_time` — the total time duration (in nanoseconds) for which tasks in a cgroup have been throttled. Maps to `cpu.throttling.throttled_time`

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-cpu)

##### ex. `/sys/fs/cgroup/cpu/docker/.../cpu.stat`

```
nr_periods 0
nr_throttled 0
throttled_time 0
```

