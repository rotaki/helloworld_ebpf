use std::io::Read;

use aya::{maps::RingBuf, programs::KProbe};
use kprobe_ringbuffer_common::DataType;
#[rustfmt::skip]
use log::{debug, warn};
use tokio::signal;

struct PollFd<T>(T);
fn poll_fd<T>(t: T) -> PollFd<T> {
    PollFd(t)
}
impl<T> PollFd<T> {
    fn readable(&mut self) -> Guard<'_, T> {
        Guard(self)
    }
}
struct Guard<'a, T>(&'a mut PollFd<T>);
impl<T> Guard<'_, T> {
    fn inner_mut(&mut self) -> &mut T {
        let Guard(PollFd(t)) = self;
        t
    }
    fn clear_ready(&mut self) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {}", ret);
    }

    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Bpf::load_file` instead.
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/kprobe-ringbuffer"
    )))?;
    if let Err(e) = aya_log::EbpfLogger::init(&mut ebpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {}", e);
    }
    let program: &mut KProbe = ebpf.program_mut("kprobe_ringbuffer").unwrap().try_into()?;
    program.load()?;
    program.attach("__x64_sys_execve", 0)?;

    // let ctrl_c = signal::ctrl_c();
    // println!("Waiting for Ctrl-C...");
    // ctrl_c.await?;
    // println!("Exiting...");

    let map = ebpf.map_mut("RING_BUFFER").unwrap();
    let ring_buf = RingBuf::try_from(map)?;
    let mut poll = poll_fd(ring_buf);

    loop {
        let mut guard = poll.readable();
        let ring_buf = guard.inner_mut();
        while let Some(event) = ring_buf.next() {
            print_event(&DataType::from_bytes(&*event));
        }
        guard.clear_ready();
    }

    Ok(())
}

fn print_event(event: &kprobe_ringbuffer_common::DataType) {
    println!(
        "pid: {}, uid: {}, command: {}, message: {}",
        event.pid,
        event.uid,
        String::from_utf8_lossy(&event.command),
        String::from_utf8_lossy(&event.message),
    );
}
