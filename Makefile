.PHONY: run
.SUFFIXES:
SHELL = /bin/bash

target/release/init: src/init/bin/*.rs
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)
	touch $@

target/release/fs-writer: src/fs-writer/bin/*.rs
	cargo build --release --bin $(@F)

rootfs.ext4: target/release/init
	mkdir -p mntdir
	if [ ! -f rootfs.ext4 ]; then truncate -s 50M $@ && mkfs.ext4 $@; fi
	sudo mount $@ mntdir
	sudo cp -t mntdir artifacts/strace artifacts/busybox
	sudo cp target/x86_64-unknown-linux-musl/release/init mntdir/init
	sudo mkdir -p mntdir/{dev,sys,proc}
	sudo umount mntdir
	touch $@

output.ext4:
	truncate -s 350M output.ext4
	mkfs.ext4 output.ext4

run: target/release/fs-writer rootfs.ext4 output.ext4
	./target/release/fs-writer disk.tar.gz output.ext4
