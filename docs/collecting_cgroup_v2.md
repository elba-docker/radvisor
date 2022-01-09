# Runtime Statistics Collection - cgroup v2

> **Note**: this document contains information about the cgroup v2 collector implementation. For information about the statistics collection mechanisms used with cgroup v1, see collecting.md.

Docker prepares individual cgroups for each container, and these are mounted (by default) at `/sys/fs/cgroup/system.slice/docker-<container id>.scope` when using systemd as the cgroup driver.

As with cgroup v1, network transfer amounts is out-of-scope of this tool (even for cgroup v2), since instrumenting network utilization requires an entirely different mechanism than the one used for block (disk) I/O, CPU, and memory.

## Statistics collected

The following fields are collected for each log line in the target log files:

- `read`
- `pids.current`
- `pids.max`
- `cpu.stat/usage_usec`
- `cpu.stat/system_usec`
- `cpu.stat/user_usec`
- `cpu.stat/nr_periods`
- `cpu.stat/nr_throttled`
- `cpu.stat/throttled_usec`
- `memory.current`
- `memory.high`
- `memory.max`
- `memory.stat/anon`
- `memory.stat/file`
- `memory.stat/kernel_stack`
- `memory.stat/pagetables`
- `memory.stat/percpu`
- `memory.stat/sock`
- `memory.stat/shmem`
- `memory.stat/file_mapped`
- `memory.stat/file_dirty`
- `memory.stat/file_writeback`
- `memory.stat/swapcached`
- `memory.stat/inactive_anon`
- `memory.stat/active_anon`
- `memory.stat/inactive_file`
- `memory.stat/active_file`
- `memory.stat/unevictable`
- `memory.stat/pgfault`
- `memory.stat/pgmajfault`
- `io.stat/rbytes`
- `io.stat/wbytes`
- `io.stat/rios`
- `io.stat/wios`
- `io.stat/dbytes`
- `io.stat/dios`

Most of these fields are straightforward, as they directly correspond to a field in a cgroup accounting file (when in the format of `<file>/<field>`, such as `cup.stat/usage_usec`). Alternatively, some fields come from cgroup accounting files that contain a single field, such as `pids.current` and `pids.max`. Information about what these fields specifically mean can be found in the [documentation for cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html).

The only fields that require discussion are:

- `read` - this is the timestamp of the log line, as a nanosecond Unix timestamp
- `io.stat/*` - these fields all come from the `io.stat` file, except the valuses are added together among all devices to produce a single value for each field.
