#![no_std]

/*
pub enum ThreadFunc {
    Create, Detach, Exit, Join,
}

#[cfg(feature = "user")]
impl ThreadFunc {
    pub fn name(self) -> &'static str {
        match self {
            ThreadFunc::Create  => "create",
            ThreadFunc::Detach  => "detach",
            ThreadFunc::Exit    => "exit",
            ThreadFunc::Join    => "join",
        }
    }
}

#[repr(C)]
pub struct ThreadInfo {
    pub ts: u64,            // Timestamp
    pub pid: u32,           // Process ID
    pub tid: u32,           // Thread ID
    pub comm: [u8; 16],     // Command Name
    pub target: u64,        // Target thread
    pub func: ThreadFunc,   // Thread function
}
*/

#[repr(C)]
pub struct DataType {
    pub pid: u32,
    pub uid: u32,
    pub command: [u8; 16],
    pub message: [u8; 12],
}

impl Default for DataType {
    fn default() -> Self {
        Self {
            pid: 0,
            uid: 0,
            command: [0; 16],
            message: [0; 12],
        }
    }
}

impl DataType {
    pub fn new(pid: u32, uid: u32, command: [u8; 16], message: [u8; 12]) -> Self {
        Self {
            pid,
            uid,
            command,
            message,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut pid = [0; 4];
        let mut uid = [0; 4];
        let mut command = [0; 16];
        let mut message = [0; 12];

        pid.copy_from_slice(&bytes[0..4]);
        uid.copy_from_slice(&bytes[4..8]);
        command.copy_from_slice(&bytes[8..24]);
        message.copy_from_slice(&bytes[24..36]);

        Self {
            pid: u32::from_le_bytes(pid),
            uid: u32::from_le_bytes(uid),
            command,
            message,
        }
    }
}
