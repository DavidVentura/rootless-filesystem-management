use clap::{ArgAction, Parser};
use firecracker_spawn::{Disk, SerialOut, Vm};
use log::{debug, error, info, trace, LevelFilter};
use simple_logger::SimpleLogger;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{sink, Seek, SeekFrom, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

mod utils;

#[derive(Parser, Default, Debug)]
struct Arguments {
    #[arg(
        short,
        long,
        help = "Path to alternative rootfs, the default expects a .tar.gz input file and will unpack it to out_fs"
    )]
    root_fs: Option<PathBuf>,
    #[arg(short, long)]
    in_file: PathBuf,
    #[arg(short, long)]
    out_fs: PathBuf,
    #[arg(
        long,
        action,
        help = "Pad input file with zeroes if necessary",
        long_help = "Input files must be multiples of 512 bytes, this option will append zeroes at the end to reach the desired length"
    )]
    pad_input_with_zeroes: bool,
    #[arg(short, action = ArgAction::Count, help = "Verbosity level")]
    verbose: u8,
}

#[derive(Debug)]
enum AppError {
    BadFs(String),
    BadInputFile(String),
}
impl Error for AppError {}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            AppError::BadFs(e) => format!("Could not detect a valid filesystem on file '{}'", e),
            AppError::BadInputFile(e) => format!(
                "Input file ({}) must be a multiple of 512 bytes, refusing to continue. Pass --pad-input-with-zeroes to get the file fixed",
                e),
        };
        write!(f, "{}", msg)
    }
}

fn setup_logging(verbose: u8) {
    let level = match verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    SimpleLogger::new()
        .with_level(level)
        .with_module_level("vmm", LevelFilter::Warn)
        .init()
        .unwrap();
}

#[derive(Debug)]
struct LogAdapter {
    line_so_far: Vec<u8>,
}

impl Write for LogAdapter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // The serial device that calls write will do write + flush for every character
        // so this function aggregates the characters and only flushes once per newline
        for c in buf {
            if *c == b'\n' {
                trace!("[K] {}", std::str::from_utf8(&self.line_so_far).unwrap());
                self.line_so_far.clear();
            } else {
                self.line_so_far.push(*c);
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const KERNEL_BYTES: &[u8] = include_bytes!("../../../artifacts/vmlinux.gz");
const ROOTFS_BYTES: &[u8] = include_bytes!("../../../artifacts/bootstrap-rootfs.ext4");
fn run(args: Arguments) -> Result<(), Box<dyn Error>> {
    setup_logging(args.verbose);
    debug!("Initializing");
    debug!("Unpacking kernel");
    let kernel = utils::gz_buf_to_fd(KERNEL_BYTES)?;

    debug!("Identifying target fs");
    let fs = utils::identify_fs(&mut File::open(&args.out_fs)?)?;
    if fs.is_none() {
        return Err(Box::new(AppError::BadFs(
            args.out_fs.into_os_string().into_string().unwrap(),
        )));
    }
    let fs = fs.unwrap();
    debug!("Detected {} as output", fs);

    let bytes_over_sector = utils::bytes_after_last_sector(&args.in_file)?;

    if bytes_over_sector > 0 {
        if args.pad_input_with_zeroes {
            info!("Padding file");
            utils::pad_file(&args.in_file, 512 - bytes_over_sector)?;
        } else {
            return Err(Box::new(AppError::BadInputFile(
                args.in_file.into_os_string().into_string().unwrap(),
            )));
        }
    }

    let mut n = NamedTempFile::new()?;
    let root_fs = args.root_fs.unwrap_or_else(|| {
        debug!("Unpacking bootstrap rootfs");
        _ = n.write(ROOTFS_BYTES).unwrap();
        n.seek(SeekFrom::Start(0)).unwrap();
        n.path().to_path_buf()
    });

    info!("Starting VM");
    let output: Box<dyn SerialOut> = if args.verbose > 1 {
        Box::new(LogAdapter {
            line_so_far: vec![],
        })
    } else {
        Box::new(sink())
    };
    run_vm(kernel, root_fs, args.in_file, args.out_fs, fs, output)?;
    Ok(())
}

fn main() {
    let args = Arguments::parse();
    match run(args) {
        Ok(()) => info!("Success"),
        Err(b) => {
            let e = b.downcast::<AppError>();
            if e.is_ok() {
                error!("{}", e.unwrap());
                std::process::exit(1);
            }
            error!("Unexpected error: {:#?}", e);
            std::process::exit(2);
        }
    };
}

fn run_vm(
    kernel: File,
    rootfs: PathBuf,
    disk_in: PathBuf,
    disk_out: PathBuf,
    fstype: utils::Filesystem,
    output: Box<dyn SerialOut>,
) -> Result<(), Box<dyn Error>> {
    let cmd = format!(
        "quiet mitigations=off panic=-1 reboot=t init=/init RUST_BACKTRACE=1 -- /dev/vdb /dev/vdc {}",
        fstype
    );
    let v = Vm {
        vcpu_count: 1,
        mem_size_mib: 128,
        kernel,
        kernel_cmdline: cmd.to_string(),
        rootfs: Disk {
            path: rootfs,
            read_only: true,
        },
        extra_disks: vec![
            Disk {
                path: disk_in,
                read_only: true,
            },
            Disk {
                path: disk_out,
                read_only: false,
            },
        ],
        net_config: None,
        use_hugepages: false,
    };
    v.make(output)?;
    Ok(())
}
