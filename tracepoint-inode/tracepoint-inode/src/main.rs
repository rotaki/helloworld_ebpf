use std::{
    fs::{File, OpenOptions},
    hint::black_box,
    io::Write,
    num,
    os::unix::fs::OpenOptionsExt,
    time::Instant,
};

use aya::{
    maps::{HashMap, Map, MapData},
    programs::TracePoint,
    Ebpf, Pod,
};
#[rustfmt::skip]
use rand::seq::SliceRandom;
use std::{io, os::unix::io::AsRawFd};

use clap::{Parser, ValueEnum};
use hdrhistogram::Histogram;
use libc;
use tokio::signal;
use tracepoint_inode_common::EventInfo;

const BLOCK_SIZE: usize = 4096;

#[derive(Copy, Clone)]
#[repr(transparent)]
struct EventInfoWrapper(EventInfo);
unsafe impl Pod for EventInfoWrapper {}

//
// Command line argument parsing
//
#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Which function to measure.
    #[clap(value_enum, long, default_value = "direct")]
    function: Function,

    /// Fallocate flag.
    #[clap(value_enum, long, default_value = "keep-size")]
    fallocate_flag: FallocateFlag,

    /// Total number of block writes.
    #[clap(short, long, default_value_t = 10000)]
    num_blocks: usize,
}

impl std::fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, fallocate_flag: {}, num_blocks: {}",
            self.function, self.fallocate_flag, self.num_blocks
        )
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum Function {
    SequentialWithKPC,
    SequentialWithKPCWithFallocate,
    SequentialDirect,
    SequentialDirectWithFallocate,
    SequentialDirectDoubleWrites,
    RandomDirect,
    RandomDirectWithFallocate,
    RandomDirectDoubleWrites,
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::SequentialWithKPC => write!(f, "sequential_with_kpc"),
            Function::SequentialWithKPCWithFallocate => {
                write!(f, "sequential_with_kpc_with_fallocate")
            }
            Function::SequentialDirect => write!(f, "sequential_direct"),
            Function::SequentialDirectWithFallocate => {
                write!(f, "sequential_direct_with_fallocate")
            }
            Function::SequentialDirectDoubleWrites => write!(f, "sequential_direct_double_writes"),
            Function::RandomDirect => write!(f, "random_direct"),
            Function::RandomDirectWithFallocate => write!(f, "random_direct_with_fallocate"),
            Function::RandomDirectDoubleWrites => write!(f, "random_direct_double_writes"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum FallocateFlag {
    None,
    ZeroRange,
    KeepSize,
}

impl std::fmt::Display for FallocateFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallocateFlag::None => write!(f, "none"),
            FallocateFlag::ZeroRange => write!(f, "zero_range"),
            FallocateFlag::KeepSize => write!(f, "keep_size"),
        }
    }
}

impl FallocateFlag {
    fn to_int(&self) -> i32 {
        match self {
            FallocateFlag::None => 0,
            FallocateFlag::ZeroRange => libc::FALLOC_FL_ZERO_RANGE,
            FallocateFlag::KeepSize => libc::FALLOC_FL_KEEP_SIZE,
        }
    }
}

//
// Utility functions
//
fn timestamp() -> u64 {
    let current_time_in_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    current_time_in_ns as u64
}

fn file_name(function: &Function) -> String {
    format!("file-{}-{}.dat", function, timestamp())
}

fn create_file(ts: u64) -> File {
    let file_path = format!("test-{}.txt", ts);
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(file_path)
        .unwrap_or_else(|e| {
            eprintln!("open error: {}", e);
            std::process::exit(1);
        })
}

fn write_block_to_file(file: &mut File, block: &[u8]) {
    file.write_all(block).unwrap_or_else(|e| {
        eprintln!("write error: {}", e);
        std::process::exit(1);
    })
}

fn fdatasync_file(file: &mut File) {
    file.sync_data().unwrap_or_else(|e| {
        eprintln!("fdatasync error: {}", e);
        std::process::exit(1);
    })
}

fn fsync_file(file: &mut File) {
    file.sync_all().unwrap_or_else(|e| {
        eprintln!("flush error: {}", e);
        std::process::exit(1);
    })
}

fn fsync_file_fd(fd: i32) -> io::Result<()> {
    let ret = unsafe { libc::fsync(fd) };
    if ret != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

//
// Common histogram printing helper
//
fn print_histogram_stats(
    histogram: &Histogram<u64>,
    step: u64,
    min_threshold: Option<u64>,
    max_threshold: Option<u64>,
) {
    plot_histogram(histogram, step, min_threshold, max_threshold);
    println!("Mean: {}", histogram.mean());
    println!("25th percentile: {}", histogram.value_at_quantile(0.25));
    println!("Median: {}", histogram.value_at_quantile(0.5));
    println!("75th percentile: {}", histogram.value_at_quantile(0.75));
}

pub fn plot_histogram(
    histogram: &Histogram<u64>,
    step: u64,
    min_threshold: Option<u64>,
    max_threshold: Option<u64>,
) {
    let max_bar_width = 50;
    let max_count = histogram.len();

    println!("{:>10} | {:<50} | {}", "Value", "Histogram", "Count(%)");
    println!("{}", "-".repeat(10 + 3 + 50 + 3 + 5 + 1 + 10));

    if let Some(min) = min_threshold {
        let percentage = histogram.percentile_below(min);
        let bar_len = ((percentage / 100.0) * max_bar_width as f64).round() as usize;
        let bar = "*".repeat(bar_len);
        println!(
            "< {:>8} | {:<50} | {:10}({:.2})",
            min,
            bar,
            (percentage / 100.0 * max_count as f64) as usize,
            percentage / 100.0
        );
    }

    for iv in histogram.iter_linear(step) {
        if let Some(min) = min_threshold {
            if iv.value_iterated_to() < min {
                continue;
            }
        }
        if let Some(max) = max_threshold {
            if iv.value_iterated_to() > max {
                break;
            }
        }
        let count = iv.count_since_last_iteration();
        let bar_len = ((count as f64 / max_count as f64) * max_bar_width as f64).round() as usize;
        let bar = "*".repeat(bar_len);
        println!(
            "{:>10} | {:<50} | {:10}({:.2})",
            iv.value_iterated_to(),
            bar,
            count,
            count as f64 / max_count as f64
        );
    }

    if let Some(max) = max_threshold {
        let percentage = 100.0 - histogram.percentile_below(max);
        let bar_len = ((percentage / 100.0) * max_bar_width as f64).round() as usize;
        let bar = "*".repeat(bar_len);
        println!(
            "> {:>8} | {:<50} | {:10}({:.2})",
            max,
            bar,
            (percentage / 100.0 * max_count as f64) as usize,
            percentage / 100.0
        );
    }
}

//
// Measured functions
// Each function now creates a histogram by timing its block write operations,
// and uses the provided number of blocks.
fn sequential_with_kpc(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let ts = timestamp();
    print_hashmap("After generating timestamp", &hashmap);
    let mut file = create_file(ts);
    print_hashmap("After creating file", &hashmap);

    let block = [1u8; BLOCK_SIZE];
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");

    for _ in 0..args.num_blocks {
        let start = Instant::now();
        write_block_to_file(&mut file, &block);
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
    }
    print_hashmap("After writing blocks to file", &hashmap);
    fsync_file(&mut file);
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn sequential_with_kpc_with_fallocate(
    ebpf: &mut Ebpf,
    args: &Args,
) -> anyhow::Result<Histogram<u64>> {
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let ts = timestamp();
    print_hashmap("After generating timestamp", &hashmap);
    let mut file = create_file(ts);
    print_hashmap("After creating file", &hashmap);

    let file_size = BLOCK_SIZE * args.num_blocks;
    let fd = file.as_raw_fd();
    unsafe {
        let ret = libc::fallocate(
            fd,
            args.fallocate_flag.to_int(),
            0,
            file_size as libc::off_t,
        );
        if ret != 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After fallocate to file", &hashmap);

    let block = [1u8; BLOCK_SIZE];
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");

    for _ in 0..args.num_blocks {
        let start = Instant::now();
        write_block_to_file(&mut file, &block);
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
    }
    print_hashmap("After writing blocks to file", &hashmap);
    fsync_file(&mut file);
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn sequential_direct(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::SequentialDirect);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let fd = file.as_raw_fd();
    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];
    for i in 0..args.num_blocks {
        let offset = (i * BLOCK_SIZE) as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After writing blocks with pwrite", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn sequential_direct_with_fallocate(
    ebpf: &mut Ebpf,
    args: &Args,
) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::SequentialDirectWithFallocate);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let file_size = BLOCK_SIZE * args.num_blocks;
    let fd = file.as_raw_fd();
    unsafe {
        let ret = libc::fallocate(
            fd,
            args.fallocate_flag.to_int(),
            0,
            file_size as libc::off_t,
        );
        if ret != 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After fallocate to file", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);

    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];
    for i in 0..args.num_blocks {
        let offset = (i * BLOCK_SIZE) as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After writing blocks with pwrite", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn sequential_direct_double_writes(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::SequentialDirectDoubleWrites);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let fd = file.as_raw_fd();
    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];
    for i in 0..args.num_blocks {
        let offset = (i * BLOCK_SIZE) as libc::off_t;
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset,
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("(1) After writing blocks with pwrite", &hashmap);
    // Seek back to the beginning of the file
    let ret = unsafe { libc::lseek(fd, 0, libc::SEEK_SET) };
    if ret < 0 {
        return Err(io::Error::last_os_error().into());
    }
    let block: [u8; BLOCK_SIZE] = [2u8; BLOCK_SIZE];
    for i in 0..args.num_blocks {
        let offset = (i * BLOCK_SIZE) as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("(2) After writing blocks with pwrite", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn random_direct(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::RandomDirect);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let fd = file.as_raw_fd();
    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];

    let mut permutation = (0..args.num_blocks).collect::<Vec<usize>>();
    permutation.shuffle(&mut rand::rng());

    for rand in permutation {
        let offset =
            (rand as u32 % args.num_blocks as u32) as libc::off_t * BLOCK_SIZE as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset as libc::off_t,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After writing blocks with pwrite", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn random_direct_with_fallocate(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::RandomDirectWithFallocate);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let file_size = BLOCK_SIZE * args.num_blocks;
    let fd = file.as_raw_fd();
    unsafe {
        let ret = libc::fallocate(
            fd,
            args.fallocate_flag.to_int(),
            0,
            file_size as libc::off_t,
        );
        if ret != 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After fallocate to file", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);

    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];
    // create a permutation of (0..args.num_blocks) to simulate random writes
    let mut permutation: Vec<usize> = (0..args.num_blocks).collect();
    permutation.shuffle(&mut rand::rng());

    for rand in permutation {
        let offset =
            (rand as u32 % args.num_blocks as u32) as libc::off_t * BLOCK_SIZE as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset as libc::off_t,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("After writing blocks with pwrite", &hashmap);
    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

fn random_direct_double_writes(ebpf: &mut Ebpf, args: &Args) -> anyhow::Result<Histogram<u64>> {
    let mut histogram =
        Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).expect("failed to create histogram");
    let map = ebpf.map_mut("EXT4INODE").unwrap();
    let hashmap: HashMap<_, EventInfoWrapper, u64> = HashMap::try_from(map)?;
    print_hashmap("Initial", &hashmap);
    let file_path = file_name(&Function::RandomDirectDoubleWrites);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(libc::O_DIRECT)
        .open(&file_path)?;
    print_hashmap("After creating file with O_DIRECT", &hashmap);

    let fd = file.as_raw_fd();
    let block: [u8; BLOCK_SIZE] = [1u8; BLOCK_SIZE];

    let mut permutation = (0..args.num_blocks).collect::<Vec<usize>>();
    permutation.shuffle(&mut rand::rng());

    for rand in permutation {
        let offset =
            (rand as u32 % args.num_blocks as u32) as libc::off_t * BLOCK_SIZE as libc::off_t;
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset as libc::off_t,
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("(1) After writing blocks with pwrite", &hashmap);

    let block = [2u8; BLOCK_SIZE];
    let mut permutation = (0..args.num_blocks).collect::<Vec<usize>>();
    permutation.shuffle(&mut rand::rng());

    for rand in permutation {
        let offset =
            (rand as u32 % args.num_blocks as u32) as libc::off_t * BLOCK_SIZE as libc::off_t;
        let start = Instant::now();
        let ret = unsafe {
            libc::pwrite(
                fd,
                block.as_ptr() as *const libc::c_void,
                BLOCK_SIZE,
                offset as libc::off_t,
            )
        };
        let elapsed = start.elapsed().as_nanos();
        histogram.record(elapsed as u64).unwrap();
        if ret < 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    print_hashmap("(2) After writing blocks with pwrite", &hashmap);

    fsync_file_fd(fd)?;
    print_hashmap("After fsyncing file", &hashmap);
    Ok(histogram)
}

//
// Helper for printing hashmap contents
//
fn print_hashmap(title: &str, hashmap: &HashMap<&mut aya::maps::MapData, EventInfoWrapper, u64>) {
    println!("**************** {} ***************", title);
    let mut count = 0;
    let mut iter = hashmap.iter();
    while let Some(Ok((key, value))) = iter.next() {
        count += 1;
        if key.0.comm.starts_with(b"code") {
            continue;
        }
        println!("key: {:?}, value: {:?}", key.0, value);
    }
    if count == 0 {
        println!("No entries in the hashmap");
    }
}

//
// Main
//
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    println!("User process ID: {}", std::process::id());

    // Bump memlock rlimit.
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        log::debug!("remove limit on locked memory failed, ret is: {}", ret);
    }

    // Parse command-line arguments.
    let args = Args::parse();
    println!("Running function: {}", args);

    // Load your eBPF program.
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/tracepoint-inode"
    )))?;
    if let Err(e) = aya_log::EbpfLogger::init(&mut ebpf) {
        log::warn!("failed to initialize eBPF logger: {}", e);
    }
    let program1: &mut TracePoint = ebpf
        .program_mut("tracepoint_ext4_mark_inode_dirty")
        .unwrap()
        .try_into()?;
    program1.load()?;
    program1.attach("ext4", "ext4_mark_inode_dirty")?;

    let program2: &mut TracePoint = ebpf
        .program_mut("tracepoint_ext4_allocate_blocks")
        .unwrap()
        .try_into()?;
    program2.load()?;
    program2.attach("ext4", "ext4_allocate_blocks")?;

    let program3: &mut TracePoint = ebpf
        .program_mut("tracepoint_jbd2_write_superblock")
        .unwrap()
        .try_into()?;
    program3.load()?;
    program3.attach("jbd2", "jbd2_write_superblock")?;

    let program4: &mut TracePoint = ebpf
        .program_mut("tracepoint_ext4_ext_map_blocks_enter")
        .unwrap()
        .try_into()?;
    program4.load()?;
    program4.attach("ext4", "ext4_ext_map_blocks_enter")?;

    let program5: &mut TracePoint = ebpf
        .program_mut("tracepoint_ext4_alloc_da_blocks")
        .unwrap()
        .try_into()?;
    program5.load()?;
    program5.attach("ext4", "ext4_alloc_da_blocks")?;

    // let program4: &mut TracePoint = ebpf.program_mut("tracepoint_block_rq_issue").unwrap().try_into()?;
    // program4.load()?;
    // program4.attach("block", "block_rq_issue")?;

    // Run the desired function and get its histogram.
    let histogram = match args.function {
        Function::SequentialWithKPC => sequential_with_kpc(&mut ebpf, &args)?,
        Function::SequentialWithKPCWithFallocate => {
            sequential_with_kpc_with_fallocate(&mut ebpf, &args)?
        }
        Function::SequentialDirect => sequential_direct(&mut ebpf, &args)?,
        Function::SequentialDirectWithFallocate => {
            sequential_direct_with_fallocate(&mut ebpf, &args)?
        }
        Function::SequentialDirectDoubleWrites => {
            sequential_direct_double_writes(&mut ebpf, &args)?
        }
        Function::RandomDirect => random_direct(&mut ebpf, &args)?,
        Function::RandomDirectWithFallocate => random_direct_with_fallocate(&mut ebpf, &args)?,
        Function::RandomDirectDoubleWrites => random_direct_double_writes(&mut ebpf, &args)?,
    };

    // In main, print the histogram.
    // (Adjust thresholds and step as needed.)
    print_histogram_stats(&histogram, 1000, Some(10000), Some(50000));

    Ok(())
}
