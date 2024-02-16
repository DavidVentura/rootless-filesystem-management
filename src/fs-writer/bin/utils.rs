use flate2::read::GzDecoder;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub(crate) enum Filesystem {
    Ext4,
    Xfs,
    Btrfs,
}
impl fmt::Display for Filesystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str_ = match self {
            Filesystem::Ext4 => "ext4",
            Filesystem::Xfs => "xfs",
            Filesystem::Btrfs => "btrfs",
        };
        write!(f, "{}", str_)
    }
}

pub(crate) fn identify_fs<R: Read>(reader: &mut R) -> Result<Option<Filesystem>, std::io::Error> {
    let ext4_magic = [0x53, 0xEF];
    let xfs_magic = "XFSB".as_bytes();
    let btrfs_magic = "_BHRfS_M".as_bytes();

    let mut buf = vec![0; 0x10_100]; // 64KiB
    reader.read_exact(&mut buf)?;

    // https://righteousit.wordpress.com/2018/05/21/xfs-part-1-superblock/
    // The superblock is at 0x000, within which the magic is at 0x00 and is 4 bytes
    let maybe_xfs_magic = buf[0..4].to_vec();

    // https://ext4.wiki.kernel.org/index.php/Ext4_Disk_Layout#The_Super_Block
    // The superblock is at 0x400, within which the magic is at 0x38 and is 2 bytes
    let maybe_ext4_magic = buf[0x438..0x43A].to_vec();

    // https://archive.kernel.org/oldwiki/btrfs.wiki.kernel.org/index.php/On-disk_Format.html#Superblock
    // The superblock is at 0x10_000, within which the magic is at 0x40 and is 8 bytes
    let maybe_btrfs_magic = buf[0x10_040..0x10_048].to_vec();

    if maybe_xfs_magic == xfs_magic {
        return Ok(Some(Filesystem::Xfs));
    }
    if maybe_ext4_magic == ext4_magic {
        return Ok(Some(Filesystem::Ext4));
    }
    if maybe_btrfs_magic == btrfs_magic {
        return Ok(Some(Filesystem::Btrfs));
    }
    Ok(None)
}

pub(crate) fn bytes_after_last_sector(in_disk: &PathBuf) -> Result<u64, std::io::Error> {
    let block_size = 512;
    let disk_size = File::open(in_disk)?.seek(SeekFrom::End(0))?;

    Ok(disk_size % block_size)
}

pub(crate) fn pad_file(in_disk: &PathBuf, bytes_to_pad: u64) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().append(true).open(in_disk)?;

    let vec: Vec<u8> = vec![0; bytes_to_pad as usize];
    _ = file.write(&vec)?;
    Ok(())
}

pub(crate) fn gz_buf_to_fd(buf: &[u8]) -> Result<File, Box<dyn Error>> {
    let opts = memfd::MemfdOptions::default().allow_sealing(true);
    let mfd = opts.create("kernel")?;

    let mut gz = GzDecoder::new(buf);
    let mut dec_buf = Vec::new();
    gz.read_to_end(&mut dec_buf)?;

    mfd.as_file().set_len(dec_buf.len() as u64)?;
    mfd.add_seals(&[memfd::FileSeal::SealShrink, memfd::FileSeal::SealGrow])?;

    // Prevent further sealing changes.
    mfd.add_seal(memfd::FileSeal::SealSeal)?;
    let mut f = mfd.into_file();

    _ = f.write(&dec_buf)?;
    f.seek(std::io::SeekFrom::Start(0))?;
    Ok(f)
}

#[cfg(test)]
mod tests {
    use crate::utils::{identify_fs, Filesystem};
    use std::fs::File;
    use std::path::PathBuf;
    #[test]
    fn it_parses_filesystems() {
        for (fname, expected) in [
            ("fs.ext4", Filesystem::Ext4),
            ("fs.xfs", Filesystem::Xfs),
            ("fs.btrfs", Filesystem::Btrfs),
        ] {
            let mut p = PathBuf::from("../../artifacts/test_artifacts/");
            p.push(fname);
            let mut f = File::open(&p).unwrap();
            let res = identify_fs(&mut f).unwrap().unwrap();
            assert!(res == expected)
        }
    }
}
