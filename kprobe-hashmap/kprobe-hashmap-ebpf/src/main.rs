#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::BPF_F_NO_PREALLOC,
    macros::{kprobe, map},
    maps::HashMap,
    programs::ProbeContext,
    EbpfContext,
};
use aya_log_ebpf::info;

#[map]
static COUNTER_TABLE: HashMap<u32, u64> =
    HashMap::<u32, u64>::with_max_entries(1024, BPF_F_NO_PREALLOC);

#[kprobe]
pub fn kprobe_hashmap(ctx: ProbeContext) -> u32 {
    match try_kprobe_hashmap(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_kprobe_hashmap(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "kprobe called");
    let tgid = ctx.tgid();
    let current = unsafe { COUNTER_TABLE.get(&tgid) };
    let res = match current {
        Some(count) => {
            let new_count = count + 1;
            COUNTER_TABLE.insert(&tgid, &new_count, 0)
        }
        None => COUNTER_TABLE.insert(&tgid, &1, 0),
    };
    match res {
        Ok(_) => Ok(0),
        Err(_) => Err(1),
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
