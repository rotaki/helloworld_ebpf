# helloworld

## Prerequisites

1. stable rust toolchains: `rustup toolchain install stable`
1. nightly rust toolchains: `rustup toolchain install nightly --component rust-src`
1. (if cross-compiling) rustup target: `rustup target add ${ARCH}-unknown-linux-musl`
1. (if cross-compiling) LLVM: (e.g.) `brew install llvm` (on macOS)
1. (if cross-compiling) C toolchain: (e.g.) [`brew install filosottile/musl-cross/musl-cross`](https://github.com/FiloSottile/homebrew-musl-cross) (on macOS)
1. bpf-linker: `cargo install bpf-linker` (`--no-default-features` on macOS)

## Build & Run

Use `cargo build`, `cargo check`, etc. as normal. Run your program with:

```shell
cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

Cargo build scripts are used to automatically build the eBPF correctly and include it in the
program.

## Cross-compiling on macOS

Cross compilation should work on both Intel and Apple Silicon Macs.

```shell
CC=${ARCH}-linux-musl-gcc cargo build --package helloworld --release \
  --target=${ARCH}-unknown-linux-musl \
  --config=target.${ARCH}-unknown-linux-musl.linker=\"${ARCH}-linux-musl-gcc\"
```
The cross-compiled program `target/${ARCH}-unknown-linux-musl/release/helloworld` can be
copied to a Linux server or VM and run there.


## Memo

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