#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
    EbpfContext,
};
use tracepoint_inode_common::{EventInfo, TracingEvent};

#[map]
static EXT4INODE: HashMap<EventInfo, u64> = HashMap::with_max_entries(1024, 0);

#[tracepoint]
pub fn tracepoint_ext4_mark_inode_dirty(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::Ext4MarkInodeDirty) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[tracepoint]
pub fn tracepoint_ext4_allocate_blocks(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::Ext4AllocateBlocks) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[tracepoint]
pub fn tracepoint_ext4_ext_map_blocks_enter(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::Ext4ExtMapBlocksEnter) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[tracepoint]
pub fn tracepoint_ext4_alloc_da_blocks(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::Ext4AllocDaBlocks) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[tracepoint]
pub fn tracepoint_jbd2_write_superblock(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::Jbd2WriteSuperblock) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

#[tracepoint]
pub fn tracepoint_block_rq_issue(ctx: TracePointContext) -> u32 {
    match try_tracepoint(ctx, TracingEvent::BlockRqIssue) {
        Ok(ret) => ret,
        Err(ret) => ret as u32,
    }
}

fn try_tracepoint(ctx: TracePointContext, event: TracingEvent) -> Result<u32, i64> {
    let pid = ctx.pid();
    let tgid = ctx.tgid();

    let thread_info = EventInfo::new(pid, tgid, ctx.command()?, event);

    let value = unsafe { EXT4INODE.get(&thread_info) };
    match value {
        Some(val) => EXT4INODE.insert(&thread_info, &(val + 1), 0)?,
        None => EXT4INODE.insert(&thread_info, &1, 0)?,
    };
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
