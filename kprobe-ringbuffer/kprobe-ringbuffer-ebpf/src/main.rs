#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_probe_read_kernel},
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
    EbpfContext,
};
use aya_log_ebpf::info;
use kprobe_ringbuffer_common::DataType;

#[map]
static RING_BUFFER: RingBuf = RingBuf::with_byte_size(4096, 0);

#[kprobe]
pub fn kprobe_ringbuffer(ctx: ProbeContext) -> u32 {
    match try_kprobe_ringbuffer(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_kprobe_ringbuffer(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "kprobe called");

    let mut data = DataType::default();
    data.pid = ctx.pid();
    data.uid = ctx.uid();
    data.command = bpf_get_current_comm().map_err(|e| e as u32)?;
    unsafe {
        bpf_probe_read_kernel(&mut data.message).map_err(|e| e as u32)?;
    };

    RING_BUFFER.output(&data, 0).map_err(|e| e as u32)?;
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
