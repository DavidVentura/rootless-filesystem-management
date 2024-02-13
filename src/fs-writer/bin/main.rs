use firecracker_spawn::{Disk, Vm};
use std::env;
use std::path::PathBuf;
fn main() {
    let args: Vec<String> = env::args().collect();
    //let cmd = "quiet panic=-1 reboot=t init=/strace -- -f /init /dev/vdb /dev/vdc";
    let cmd = "quiet panic=-1 reboot=t init=/init RUST_BACKTRACE=1 -- /dev/vdb /dev/vdc";
    let v = Vm {
        vcpu_count: 1,
        mem_size_mib: 128,
        kernel_cmdline: cmd.to_string(),
        kernel_path: PathBuf::from("./artifacts/vmlinux"),
        rootfs: Disk {
            path: PathBuf::from("./rootfs.ext4"),
            read_only: true,
        },
        extra_disks: vec![
            Disk {
                path: PathBuf::from(&args[1]),
                read_only: true,
            },
            Disk {
                path: PathBuf::from(&args[2]),
                read_only: false,
            },
        ],
        net_config: None,
    };
    v.make().unwrap();
}
