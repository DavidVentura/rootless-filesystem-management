.PHONY: run test_artifacts build
.SUFFIXES:
.DEFAULT_GOAL := build

SHELL = /bin/bash
FS_ARTIFACTS = artifacts/test_artifacts/fs.ext4 artifacts/test_artifacts/fs.xfs artifacts/test_artifacts/fs.btrfs
RELEASE_DIR = target/x86_64-unknown-linux-musl/release
export CC = musl-gcc -I/usr/include -I/usr/include/x86_64-linux-gnu

target/x86_64-unknown-linux-musl/release/init: src/init/bin/*.rs
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)
	touch $@

target/x86_64-unknown-linux-musl/release/fs-writer: src/fs-writer/bin/*.rs artifacts/vmlinux.gz artifacts/bootstrap-initrd.cpio.gz
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)

artifacts/bootstrap-initrd.cpio.gz: $(RELEASE_DIR)/init
	mkdir -p initrd
	# cp -t mntdir artifacts/strace artifacts/busybox
	mkdir -p initrd/{dev,sys,proc,destination}
	strip $(RELEASE_DIR)/init
	cp $(RELEASE_DIR)/init initrd/init
	cd initrd && find . -print0 | cpio --null --create --verbose --format=newc | gzip > ../$@
	cd initrd && find . -print0 | cpio --null --create --verbose --format=newc > ../artifacts/bootstrap-initrd.cpio

output.ext4:
	truncate -s 350M output.ext4
	mkfs.ext4 output.ext4

build: $(RELEASE_DIR)/fs-writer $(RELEASE_DIR)/init
	strip $^

run: $(RELEASE_DIR)/fs-writer output.ext4
	$(RELEASE_DIR)/fs-writer --in-file disk.tar.gz --out-fs output.ext4 --pad-input-with-zeroes

test: build
	cargo test --verbose --release --target x86_64-unknown-linux-musl

test_artifacts: $(FS_ARTIFACTS)
	:
$(FS_ARTIFACTS):
	truncate -s 300M $@
	mkfs$(suffix $@) -q $@
	truncate -s 200K $@
