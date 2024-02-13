# "Rootless" filesystem management

This can unpack a `tar.gz` file into an empty `ext4`/`xfs`/`btrfs` filesystem, without using sudo to do so:

```
$ tar tvf disk.tar.gz | wc -l                                                                                                                                     
5283

$ time ./target/release/fs-writer disk.tar.gz output.ext4
real    0m0.975s

$ sudo mount output.ext4
$ sudo find output.ext4 | wc -l
5284
$ # an extra file for /lost+found

```

## How does it work

This tool creates a virtual machine with [firecracker](), adds 3 memory-mapped block devices:
- "rootfs", containing the unpacking tool (read only)
- source tar.gz file (read only)
- destination filesystem (read write)

When the VM boots, the custom init process unpacks the source tar file into the destination filesystem and exits.

This tool is comparable to [guestfish](https://libguestfs.org/guestfish.1.html)

## Why

It's dumb to need to _mount_ a filesystem to put files in it. There should be tools like [mtools](https://www.gnu.org/software/mtools/manual/mtools.html) which allow modifying the contents of an image diractly. 

This tools' approach embraces the suck, but at least it's fast.
