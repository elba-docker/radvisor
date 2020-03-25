# Runtime Statistics Collection

Runtime statistics for each container are taken from the virtual files for each container's cgroup, located at `/sys/fs/cgroup/<subsystem>/docker/<container id>/file`.

More information is available at the Docker wiki: [Runtime metrics](https://docs.docker.com/config/containers/runmetrics/).

> **Note**: while the base docker daemon collects stats for network transfer amounts, that sort of collection is out of the scope of rAdvisor (at least currently). This is due to network monitoring requiring different and significantly more involved monitoring than the various cgroup subsystems.

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
- `nr_throttled` — number of times tasks in a cgroup have been throttled (that is, not allowed to run because they have exhausted all of the available time as specified by their quota). Maps to `cpu.throttling.throttled.count`
- `throttled_time` — the total time duration (in nanoseconds) for which tasks in a cgroup have been throttled. Maps to `cpu.throttling.throttled.time`

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-cpu)

##### ex. `/sys/fs/cgroup/cpu/docker/.../cpu.stat`

```
nr_periods 0
nr_throttled 0
throttled_time 0
```

### Memory

The `memory` subsystem includes information on the memory usage and limitations of the processes running in a cgroup.

More information: [Kernel docs](https://www.kernel.org/doc/Documentation/cgroup-v1/memory.txt).

#### `memory.usage_in_bytes`

reports the total current memory usage by processes in the cgroup (in bytes). Maps to `memory.usage.current`

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.usage_in_bytes`

```
1982464
```

#### `memory.max_usage_in_bytes`

reports the maximum amount of memory and swap space used by processes in the cgroup (in bytes). Maps to `memory.usage.max`

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.max_usage_in_bytes`

```
3092480
```

#### `memory.limit_in_bytes`

maximum amount of user memory (including file cache). Maps to `memory.limit.hard`

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.limit_in_bytes`

```
9223372036854771712
```

#### `memory.soft_limit_in_bytes`

enables flexible sharing of memory. Under normal circumstances, control groups are allowed to use as much of the memory as needed, constrained only by their hard limits set with the `memory.limit_in_bytes` parameter. However, when the system detects memory contention or low memory, control groups are forced to restrict their consumption to their _soft limits_. Maps to `memory.limit.soft`

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-memory)

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.soft_limit_in_bytes`

```
9223372036854771712
```

#### `memory.failcnt`

reports the number of times that the memory limit has reached the value set in `memory.limit_in_bytes`. Maps to `memory.failcnt`

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-memory)

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.failcnt`

```
0
```

#### `memory.stat`

reports a wide range of memory statistics, as described in the following table:

| Statistic                   | Description                                                                                          |
| --------------------------- | ---------------------------------------------------------------------------------------------------- |
| `cache`                     | page cache, including tmpfs (shmem), in bytes                                                        |
| `rss`                       | anonymous and swap cache, not including tmpfs (shmem), in bytes                                      |
| `mapped_file`               | size of memory-mapped mapped files, including tmpfs (shmem), in bytes                                |
| `pgpgin`                    | number of pages paged into memory                                                                    |
| `pgpgout`                   | number of pages paged out of memory                                                                  |
| `swap`                      | swap usage, in bytes                                                                                 |
| `active_anon`               | anonymous and swap cache on active least-recently-used (LRU) list, including tmpfs (shmem), in bytes |
| `inactive_anon`             | anonymous and swap cache on inactive LRU list, including tmpfs (shmem), in bytes                     |
| `active_file`               | file-backed memory on active LRU list, in bytes                                                      |
| `inactive_file`             | file-backed memory on inactive LRU list, in bytes                                                    |
| `unevictable`               | memory that cannot be reclaimed, in bytes                                                            |
| `hierarchical_memory_limit` | memory limit for the hierarchy that contains the memory cgroup, in bytes                             |
| `hierarchical_memsw_limit`  | memory plus swap limit for the hierarchy that contains the memory cgroup, in bytes                   |

Additionally, each of these files other than `hierarchical_memory_limit` and `hierarchical_memsw_limit` has a counterpart prefixed `total_` that reports not only on the cgroup, but on all its children as well. For example, `swap` reports the swap usage by a cgroup and `total_swap` reports the total swap usage by the cgroup and all its child groups.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-memory)

> **Note** For these reasons, we use only the total values to give containers the flexibility to utilize cgroups of their own while still being able to monitor all resource utilization.

When you interpret the values reported by memory.stat, note how the various statistics inter-relate:

- `active_anon` + `inactive_anon` = anonymous memory + file cache for `tmpfs` + swap cache
  Therefore, `active_anon` + `inactive_anon` ≠ `rss`, because `rss` does not include `tmpfs`.
- `active_file` + `inactive_file` = cache - size of `tmpfs`

##### Mapping Information

| Statistic                   | Mapped To                             |
| --------------------------- | ------------------------------------- |
| `hierarchical_memory_limit` | `memory.hiearchical_limit.memory`     |
| `hierarchical_memsw_limit`  | `memory.hiearchical_limit.memoryswap` |
| `total_cache`               | `memory.cache`                        |
| `total_rss`                 | `memory.rss.all`                      |
| `total_rss_huge`            | `memory.rss.huge`                     |
| `total_mapped_file`         | `memory.mapped`                       |
| `total_swap`                | `memory.swap`                         |
| `total_pgpgin`              | `memory.paged.in`                     |
| `total_pgpgout`             | `memory.paged.out`                    |
| `total_pgfault`             | `memory.fault.total`                  |
| `total_pgmajfault`          | `memory.fault.major`                  |
| `total_inactive_anon`       | `memory.anon.inactive`                |
| `total_active_anon`         | `memory.anon.active`                  |
| `total_inactive_file`       | `memory.file.inactive`                |
| `total_active_file`         | `memory.file.active`                  |
| `total_unevictable`         | `memory.unevictable`                  |

##### ex. `/sys/fs/cgroup/memory/docker/.../memory.stat`

```
cache 192512
rss 356352
rss_huge 0
shmem 0
mapped_file 114688
dirty 0
writeback 0
pgpgin 2970
pgpgout 2836
pgfault 4211
pgmajfault 3
inactive_anon 0
active_anon 356352
inactive_file 180224
active_file 12288
unevictable 0
hierarchical_memory_limit 9223372036854771712
total_cache 192512
total_rss 356352
total_rss_huge 0
total_shmem 0
total_mapped_file 114688
total_dirty 0
total_writeback 0
total_pgpgin 2970
total_pgpgout 2836
total_pgfault 4211
total_pgmajfault 3
total_inactive_anon 0
total_active_anon 356352
total_inactive_file 180224
total_active_file 12288
total_unevictable 0
```

### Block IO

The Block I/O (`blkio`) subsystem controls and monitors access to I/O on block devices by tasks in cgroups. Writing values to some of these pseudofiles limits access or bandwidth, and reading values from some of these pseudofiles provides information on I/O operations.

All files have a `_recursive` version, which includes stats for the processes in the cgroup as well as any children cgroups.

More information: [Kernel docs](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/blkio-controller.html).

#### `blkio.io_service_bytes_recursive`

reports the number of bytes transferred to or from specific devices by a cgroup as seen by the CFQ scheduler. Entries have four fields: *major*, *minor*, *operation*, and *bytes*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, *operation* represents the type of operation (`read`, `write`, `sync`, or `async`) and *bytes* is theMajor and minor are device types and node numbers specified in Linux Allocated Devices, operation represents the type of operation (read, write, sync, or async) and 

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_service_bytes_recursive`

```
8:0 Read 34787328
8:0 Write 74403840
8:0 Sync 37494784
8:0 Async 71696384
8:0 Total 109191168
Total 109191168
```

#### `blkio.io_serviced_recursive`

reports the number of I/O operations performed on specific devices by a cgroup as seen by the CFQ scheduler. Entries have four fields: *major*, *minor*, *operation*, and *number*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, *operation* represents the type of operation (`read`, `write`, `sync`, or `async`) and *number* represents the number of operations.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_serviced_recursive`

```
8:0 Read 1736
8:0 Write 24054
8:0 Sync 15904
8:0 Async 9886
8:0 Total 25790
Total 25790
```

#### `blkio.io_service_time_recursive`

reports the total time between request dispatch and request completion for I/O operations on specific devices by a cgroup as seen by the CFQ scheduler. Entries have four fields: *major*, *minor*, *operation*, and *time*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, *operation* represents the type of operation (`read`, `write`, `sync`, or `async`) and *time* is the length of time in nanoseconds (ns). The time is reported in nanoseconds rather than a larger unit so that this report is meaningful even for solid-state devices.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_service_time_recursive`

```
8:0 Read 553729709
8:0 Write 467872175
8:0 Sync 569993511
8:0 Async 451608373
8:0 Total 1021601884
Total 1021601884
```

#### `blkio.io_queued_recursive`

reports the number of requests queued for I/O operations by a cgroup. Entries have two fields: *number* and *operation*. *Number* is the number of requests, and *operation* represents the type of operation (`read`, `write`, `sync`, or `async`).

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_queued_recursive`

```
8:0 Read 0
8:0 Write 0
8:0 Sync 0
8:0 Async 0
8:0 Total 0
Total 0
```

#### `blkio.io_wait_time_recursive`

reports the total time I/O operations on specific devices by a cgroup spent waiting for service in the scheduler queues. When you interpret this report, note:

- the time reported can be greater than the total time elapsed, because the time reported is the cumulative total of all I/O operations for the cgroup rather than the time that the cgroup itself spent waiting for I/O operations. To find the time that the group as a whole has spent waiting, use the `blkio.group_wait_time` parameter.
- if the device has a `queue_depth` > 1, the time reported only includes the time until the request is dispatched to the device, not any time spent waiting for service while the device reorders requests.

Entries have four fields: *major*, *minor*, *operation*, and *time*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, *operation* represents the type of operation (`read`, `write`, `sync`, or `async`) and *time* is the length of time in nanoseconds (ns). The time is reported in nanoseconds rather than a larger unit so that this report is meaningful even for solid-state devices.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_wait_time_recursive`

```
8:0 Read 341027463
8:0 Write 183051407147
8:0 Sync 710185876
8:0 Async 182682248734
8:0 Total 183392434610
Total 183392434610
```

#### `blkio.io_merged_recursive`

reports the number of BIOS requests merged into requests for I/O operations by a cgroup. Entries have two fields: *number* and *operation*. *Number* is the number of requests, and *operation* represents the type of operation (`read`, `write`, `sync`, or `async`). 

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.io_merged_recursive`

```
8:0 Read 112
8:0 Write 5989
8:0 Sync 112
8:0 Async 5989
8:0 Total 6101
Total 6101
```

#### `blkio.time_recursive`

reports the time that a cgroup had I/O access to specific devices. Entries have three fields: *major*, *minor*, and *time*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, and *time* is the length of time in milliseconds (ms).

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.time_recursive`

```
8:0 1677059003
```

#### `blkio.sectors_recursive`

reports the number of sectors transferred to or from specific devices by a cgroup. Entries have three fields: *major*, *minor*, and *sectors*. *Major* and *minor* are device types and node numbers specified in *Linux Allocated Devices*, and *sectors* is the number of disk sectors.

Source: [Red Hat Customer Portal](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/ch-subsystems_and_tunable_parameters#sec-blkio)

##### ex. `/sys/fs/cgroup/blkio/docker/.../blkio.sectors_recursive`

```
8:0 213264
```
