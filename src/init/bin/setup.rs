use std::ffi::CString;
use std::fs;
use std::io;
use std::path::PathBuf;

fn mount_pseudo(target: &str, fstype: &str) -> Result<(), io::Error> {
    mount(None, PathBuf::from(target), PathBuf::from(fstype))
}

pub(crate) fn mount(
    source: Option<PathBuf>,
    target: PathBuf,
    fstype: PathBuf,
) -> Result<(), io::Error> {
    let src = source
        .clone()
        .unwrap_or(PathBuf::from("none"))
        .into_os_string()
        .into_string()
        .unwrap();
    let tgt = target.into_os_string().into_string().unwrap();
    let fs = fstype.into_os_string().into_string().unwrap();
    let c_src = CString::new(src).unwrap();
    let c_tgt = CString::new(tgt).unwrap();
    let c_fstype = CString::new(fs).unwrap();

    let res = unsafe {
        libc::mount(
            c_src.as_ptr(),
            c_tgt.as_ptr(),
            c_fstype.as_ptr(),
            libc::MS_NOATIME | libc::MS_NODIRATIME,
            std::ptr::null(),
        )
    };
    if res == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn mknod(path: &str, major: u32, minor: u32, mode: u32) -> Result<(), io::Error> {
    let devnum = libc::makedev(major, minor);
    let path = CString::new(path).unwrap();
    let res = unsafe { libc::mknod(path.as_ptr(), mode | 0666, devnum) };
    if res == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn parse_uevent(s: &str) -> (String, usize, usize) {
    //open("/sys/dev/block/254:0/uevent", O_RDONLY) = 6
    //read(6, "MAJOR=254\nMINOR=0\nDEVNAME=vda\nDE"..., 127) = 53
    let mut devname = "";
    let mut major: usize = 0;
    let mut minor: usize = 0;
    for line in s.split("\n") {
        let parts: Vec<&str> = line.split("=").collect();
        if parts.len() != 2 {
            continue;
        }
        match parts[0] {
            "MAJOR" => major = parts[1].parse().unwrap(),
            "MINOR" => minor = parts[1].parse().unwrap(),
            "DEVNAME" => devname = parts[1],
            _ => (),
        }
    }
    (devname.to_string(), major, minor)
}
fn mount_blockdevs() -> Result<(), io::Error> {
    for f in glob::glob("/sys/dev/block/*/uevent").unwrap() {
        let contents = fs::read_to_string(f.unwrap())?;
        let (devname, major, minor) = parse_uevent(&contents);
        mknod(
            &format!("/dev/{devname}"),
            major as u32,
            minor as u32,
            libc::S_IFBLK,
        )?;
    }
    Ok(())
}
pub(crate) fn setup_environment() -> Result<(), io::Error> {
    mount_pseudo("/proc", "proc").unwrap();
    mount_pseudo("/sys", "sysfs").unwrap();
    mount_pseudo("/dev", "tmpfs").unwrap();
    mknod("/dev/null", 1, 3, libc::S_IFCHR)?;
    mount_blockdevs()?;
    Ok(())
}

pub(crate) fn sync() {
    unsafe {
        libc::sync();
    };
}
