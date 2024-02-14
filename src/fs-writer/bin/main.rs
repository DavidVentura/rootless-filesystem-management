use clap::Parser;
use firecracker_spawn::{Disk, Vm};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

mod utils;

#[derive(Parser, Default, Debug)]
struct Arguments {
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
fn buf_to_fd(buf: &[u8]) -> Result<File, Box<dyn Error>> {
    let opts = memfd::MemfdOptions::default().allow_sealing(true);
    let mfd = opts.create("kernel")?;

    mfd.as_file().set_len(buf.len() as u64)?;
    mfd.add_seals(&[memfd::FileSeal::SealShrink, memfd::FileSeal::SealGrow])?;

    // Prevent further sealing changes.
    mfd.add_seal(memfd::FileSeal::SealSeal)?;
    let mut f = mfd.into_file();
    f.write(buf)?;
    f.seek(std::io::SeekFrom::Start(0))?;
    Ok(f)
}
fn run(args: Arguments) -> Result<(), Box<dyn Error>> {
    let kernel = buf_to_fd(KERNEL_BYTES)?;

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

    run_vm(
        kernel,
        PathBuf::from("./rootfs.ext4"),
        args.in_file,
        args.out_fs,
        fs,
    )?;
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
    //let cmd = "quiet panic=-1 reboot=t init=/strace -- -f /init /dev/vdb /dev/vdc ext4";
    let cmd = format!(
        "quiet panic=-1 reboot=t init=/init RUST_BACKTRACE=1 -- /dev/vdb /dev/vdc {}",
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
    };
    v.make()?;
    Ok(())
}
