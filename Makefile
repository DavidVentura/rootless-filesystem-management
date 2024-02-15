.PHONY: run test_artifacts build
.SUFFIXES:
SHELL = /bin/bash
FS_ARTIFACTS = artifacts/test_artifacts/fs.ext4 artifacts/test_artifacts/fs.xfs artifacts/test_artifacts/fs.btrfs
RELEASE_DIR = target/x86_64-unknown-linux-musl/release
export CC = musl-gcc -I/usr/include -I/usr/include/x86_64-linux-gnu

target/x86_64-unknown-linux-musl/release/init: src/init/bin/*.rs
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)
	touch $@

target/x86_64-unknown-linux-musl/release/fs-writer: src/fs-writer/bin/*.rs artifacts/vmlinux artifacts/bootstrap-rootfs.ext4
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)

artifacts/bootstrap-rootfs.ext4: $(RELEASE_DIR)/init
	mkdir -p mntdir
	if [ ! -f $@ ]; then truncate -s 1M $@ && mkfs.ext4 $@; fi
	sudo mount $@ mntdir
	# sudo cp -t mntdir artifacts/strace artifacts/busybox
	sudo strip $(RELEASE_DIR)/init
	sudo cp $(RELEASE_DIR)/init mntdir/init
	sudo mkdir -p mntdir/{dev,sys,proc,destination}
	sudo umount mntdir
	touch $@

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
