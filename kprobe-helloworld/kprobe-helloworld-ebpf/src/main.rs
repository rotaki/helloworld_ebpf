#![no_std]
#![no_main]

use aya_ebpf::{macros::kprobe, programs::ProbeContext};
use aya_log_ebpf::info;

#[kprobe]
pub fn kprobe_helloworld(ctx: ProbeContext) -> u32 {
    match try_kprobe_helloworld(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_kprobe_helloworld(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "kprobe called");
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
