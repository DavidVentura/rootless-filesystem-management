use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum Filesystem {
    Ext4,
    XFS,
}
impl fmt::Display for Filesystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str_ = match self {
            Filesystem::Ext4 => "ext4",
            Filesystem::XFS => "xfs",
        };
        write!(f, "{}", str_)
    }
}

pub(crate) fn identify_fs(in_disk: &PathBuf) -> Result<Option<Filesystem>, std::io::Error> {
    let ext4_magic = [0x53, 0xEF];
    let xfs_magic = ['X' as u8, 'F' as u8, 'S' as u8, 'B' as u8];

    let mut buf = vec![0; 0x500];
    File::open(&in_disk)?.read(&mut buf)?;

    // https://righteousit.wordpress.com/2018/05/21/xfs-part-1-superblock/
    // The superblock is at 0x000, within the sb, magic is at 0x00 and is 4 bytes
    let maybe_xfs_magic = buf[0..4].to_vec();

    // https://ext4.wiki.kernel.org/index.php/Ext4_Disk_Layout#The_Super_Block
    // The superblock is at 0x400, within the sb, magic is at 0x38 and is 2 bytes
    let maybe_ext4_magic = buf[0x438..0x43A].to_vec();

    if maybe_xfs_magic == xfs_magic {
        return Ok(Some(Filesystem::XFS));
    }
    if maybe_ext4_magic == ext4_magic {
        return Ok(Some(Filesystem::Ext4));
    }
    Ok(None)
}

pub(crate) fn bytes_after_last_sector(in_disk: &PathBuf) -> Result<u64, std::io::Error> {
    let block_size = 512;
    let disk_size = File::open(&in_disk)?.seek(SeekFrom::End(0))?;

    Ok(disk_size % block_size)
}

pub(crate) fn pad_file(in_disk: &PathBuf, bytes_to_pad: u64) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().write(true).append(true).open(in_disk)?;

    let vec: Vec<u8> = vec![0; bytes_to_pad as usize];
    file.write(&vec)?;
    Ok(())
}
