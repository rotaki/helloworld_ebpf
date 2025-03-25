#![no_std]

#[derive(Copy, Clone)]
#[repr(C)]
pub struct EventInfo {
    pub pid: u32,
    pub tgid: u32,
    pub comm: [u8; 16],
    pub event: TracingEvent,
}

impl EventInfo {
    pub fn new(pid: u32, tgid: u32, comm: [u8; 16], event: TracingEvent) -> Self {
        EventInfo {
            pid,
            tgid,
            comm,
            event,
        }
    }
}

#[cfg(feature = "user")]
impl core::fmt::Debug for EventInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EventInfo")
            .field("pid", &self.pid)
            .field("tgid", &self.tgid)
            .field("comm", &core::str::from_utf8(&self.comm).unwrap())
            .field("event", &self.event)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum TracingEvent {
    Ext4MarkInodeDirty,
    Ext4AllocDaBlocks,
    Ext4AllocateBlocks,
    Ext4ExtMapBlocksEnter,
    Jbd2WriteSuperblock,
    BlockRqIssue,
}

#[cfg(feature = "user")]
impl core::fmt::Debug for TracingEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TracingEvent::Ext4MarkInodeDirty => write!(f, "Ext4MarkInodeDirty"),
            TracingEvent::Ext4AllocDaBlocks => write!(f, "Ext4AllocDaBlocks"),
            TracingEvent::Ext4AllocateBlocks => write!(f, "Ext4AllocateBlocks"),
            TracingEvent::Ext4ExtMapBlocksEnter => write!(f, "Ext4ExtMapBlocksEnter"),
            TracingEvent::Jbd2WriteSuperblock => write!(f, "Jbd2WriteSuperblock"),
            TracingEvent::BlockRqIssue => write!(f, "BlockRqIssue"),
        }
    }
}
