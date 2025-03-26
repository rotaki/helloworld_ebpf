# Notes

## Project creation and execution

```shell
cargo generate https://github.com/aya-rs/aya-template
RUST_LOG=info cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

## Tracepoints

You can list available tracepoints in your Linux system using one of the following methods:

1. **Via the DebugFS Interface:**  
   Most Linux systems have the tracepoints listed in the file:  
   ```bash
   cat /sys/kernel/debug/tracing/available_events
   ```  
   This command will print all the available tracepoints grouped by subsystem (e.g., sched, syscalls, block, etc.). Note that you may need root privileges and ensure that DebugFS is mounted (commonly at `/sys/kernel/debug`).

2. **Using bpftrace:**  
   If you have bpftrace installed, you can list tracepoints with:  
   ```bash
   sudo bpftrace -l tracepoint:*
   ```  
   This command shows a list of all tracepoints that you can attach your eBPF programs to.

Both methods provide you with the complete list of tracepoints available on your system.

The tracepoint category and event name can be found by the following command:

```shell
 find /sys/kernel/tracing/events/ -type f -name id | grep ${KEYWORD}
```

## kprobe

To find a event name where you want to attach a kprobe, use the following command:

```shell
cat /proc/kallsyms | grep -i ${KEYWORD}
```

## EbpfContext IDs

```
what are differences between these ids?

pub trait EbpfContext {
    fn as_ptr(&self) -> *mut c_void;

    #[inline]
    fn command(&self) -> Result<[u8; TASK_COMM_LEN], c_long> {
        bpf_get_current_comm()
    }

    fn pid(&self) -> u32 {
        bpf_get_current_pid_tgid() as u32
    }

    fn tgid(&self) -> u32 {
        (bpf_get_current_pid_tgid() >> 32) as u32
    }

    fn uid(&self) -> u32 {
        bpf_get_current_uid_gid() as u32
    }

    fn gid(&self) -> u32 {
        (bpf_get_current_uid_gid() >> 32) as u32
    }
}
```

Here's an ASCII diagram to illustrate the differences:

```
For process-related IDs (from bpf_get_current_pid_tgid()):
+-------------------------------------------+
|           64-bit value                    |
+---------------------+---------------------+
|    TGID (upper 32)  |   PID (lower 32)    |
|  (Thread Group ID)  |   (Thread ID)       |
+---------------------+---------------------+

For user-related IDs (from bpf_get_current_uid_gid()):
+-------------------------------------------+
|           64-bit value                    |
+---------------------+---------------------+
|    GID (upper 32)   |   UID (lower 32)    |
|   (Group ID)        |   (User ID)         |
+---------------------+---------------------+
```

**Explanation:**

- **TGID vs PID:**  
  The 64-bit integer from `bpf_get_current_pid_tgid()` is split into two parts:
  - **TGID (upper 32 bits):** Represents the Thread Group ID, which is essentially the process ID shared by all threads in the process.
  - **PID (lower 32 bits):** Represents the unique Thread ID for the current thread.

- **GID vs UID:**  
  Similarly, the 64-bit integer from `bpf_get_current_uid_gid()` is divided as:
  - **GID (upper 32 bits):** Represents the Group ID, which is related to user group permissions.
  - **UID (lower 32 bits):** Represents the User ID of the current task.

This diagram and explanation should help clarify that TGID and GID come from different 64-bit sources and represent entirely different concepts—process identification versus user group identification.

## Directly attaching to user functions

* https://eunomia.dev/en/tutorials/37-uprobe-rust/