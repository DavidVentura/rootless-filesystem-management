use std::ffi::CString;
use std::fs;
use std::io;

fn mount_pseudo(target: &str, fstype: &str) -> Result<(), io::Error> {
    mount(None, target, fstype)
}

pub(crate) fn mount(source: Option<&str>, target: &str, fstype: &str) -> Result<(), io::Error> {
    // println!("Mounting {:?} on {} as {}", source, target, fstype);
    let src = CString::new(source.unwrap_or("none")).unwrap();
    let tgt = CString::new(target).unwrap();
    let fstype_ = CString::new(fstype).unwrap();

    let res = unsafe {
        libc::mount(
            src.as_ptr(),
            tgt.as_ptr(),
            fstype_.as_ptr(),
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
