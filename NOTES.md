# Notes



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