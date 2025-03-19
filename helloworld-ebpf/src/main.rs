#![no_std]
#![no_main]

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};
use aya_log_ebpf::info;

#[tracepoint]
pub fn helloworld(ctx: TracePointContext) -> u32 {
    match try_helloworld(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_helloworld(ctx: TracePointContext) -> Result<u32, u32> {
    info!(&ctx, "Hello world!");
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
