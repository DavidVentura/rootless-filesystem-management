use clap::Parser;
use firecracker_spawn::{Disk, Vm};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

mod utils;

#[derive(Parser, Default, Debug)]
struct Arguments {
    #[arg(short, long)]
    root_fs: Option<PathBuf>,
    #[arg(short, long)]
    in_file: PathBuf,
    #[arg(short, long)]
    out_fs: PathBuf,
    #[arg(long, action)]
    pad_input_with_zeroes: bool,
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
            AppError::BadFs(e) => e.to_string(),
            AppError::BadInputFile(e) => format!(
                "Input file ({}) must be a multiple of 512 bytes, refusing to continue.\nPass --pad-input-with-zeroes to get the file fixed",
                e),
        };
        write!(f, "{}", msg)
    }
}

const KERNEL_BYTES: &[u8] = include_bytes!("../../../artifacts/vmlinux");
const ROOTFS_BYTES: &[u8] = include_bytes!("../../../artifacts/bootstrap-rootfs.ext4");
fn run(args: Arguments) -> Result<(), Box<dyn Error>> {
    let kernel = utils::buf_to_fd(KERNEL_BYTES)?;

    let fs = utils::identify_fs(&mut File::open(&args.out_fs)?)?;
    if fs.is_none() {
        return Err(Box::new(AppError::BadFs(format!(
            "Could not detect a valid filesystem on file '{}'",
            args.out_fs.into_os_string().into_string().unwrap()
        ))));
    }
    let fs = fs.unwrap();
    println!("Detected {} as output", fs);

    let bytes_over_sector = utils::bytes_after_last_sector(&args.in_file)?;

    if bytes_over_sector > 0 {
        if args.pad_input_with_zeroes {
            println!("Padding file..");
            utils::pad_file(&args.in_file, 512 - bytes_over_sector)?;
        } else {
            return Err(Box::new(AppError::BadInputFile(
                args.in_file.into_os_string().into_string().unwrap(),
            )));
        }
    }

    let mut n = NamedTempFile::new()?;
    let root_fs = args.root_fs.unwrap_or_else(|| {
        n.write(ROOTFS_BYTES).unwrap();
        n.seek(SeekFrom::Start(0)).unwrap();
        n.path().to_path_buf()
    });

    run_vm(kernel, root_fs, args.in_file, args.out_fs, fs)?;
    Ok(())
}

fn main() {
    let args = Arguments::parse();
    match run(args) {
        Ok(()) => println!("Success"),
        Err(b) => {
            let e = b.downcast::<AppError>();
            if e.is_ok() {
                println!("{}", e.unwrap());
                std::process::exit(1);
            }
            println!("Unexpected error: {:#?}", e);
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
    v.make()?;
    Ok(())
}
