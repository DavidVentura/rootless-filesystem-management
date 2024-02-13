use clap::Parser;
use firecracker_spawn::{Disk, Vm};
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::PathBuf;

#[derive(Parser, Default, Debug)]
struct Arguments {
    #[arg(short, long)]
    in_file: PathBuf,
    #[arg(short, long)]
    out_fs: PathBuf,
    #[arg(long, action)]
    pad_input_with_zeroes: bool,
}

fn bytes_after_last_sector(in_disk: &PathBuf) -> u64 {
    let block_size = 512;
    let disk_size = File::open(&in_disk)
        .unwrap()
        .seek(SeekFrom::End(0))
        .unwrap();

    disk_size % block_size
}

fn pad_file(in_disk: &PathBuf, bytes_to_pad: u64) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(in_disk)
        .unwrap();

    let vec: Vec<u8> = vec![0; bytes_to_pad as usize];
    file.write(&vec).unwrap();
}

fn main() {
    let args = Arguments::parse();

    let bytes_over_sector = bytes_after_last_sector(&args.in_file);

    if bytes_over_sector > 0 {
        if args.pad_input_with_zeroes {
            println!("Padding file..");
            pad_file(&args.in_file, 512 - bytes_over_sector);
        } else {
            println!(
                "Input file ({}) must be a multiple of 512 bytes, refusing to continue.",
                args.in_file.into_os_string().into_string().unwrap(),
            );
            println!("Pass --pad-input-with-zeroes to get the file fixed");
            return;
        }
    }

    run(
        PathBuf::from("./rootfs.ext4"),
        PathBuf::from("./artifacts/vmlinux"),
        args.in_file,
        args.out_fs,
    )
    .unwrap();
    println!("Success");
}

fn run(
    rootfs: PathBuf,
    kernel: PathBuf,
    disk_in: PathBuf,
    disk_out: PathBuf,
) -> Result<(), Box<dyn Error>> {
    //let cmd = "quiet panic=-1 reboot=t init=/strace -- -f /init /dev/vdb /dev/vdc ext4";
    let cmd = "quiet panic=-1 reboot=t init=/init RUST_BACKTRACE=1 -- /dev/vdb /dev/vdc ext4";
    let v = Vm {
        vcpu_count: 1,
        mem_size_mib: 128,
        kernel_cmdline: cmd.to_string(),
        kernel_path: kernel,
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
