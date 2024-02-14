.PHONY: run test_artifacts
.SUFFIXES:
SHELL = /bin/bash
FS_ARTIFACTS = artifacts/test_artifacts/fs.ext4 artifacts/test_artifacts/fs.xfs artifacts/test_artifacts/fs.btrfs

target/release/init: src/init/bin/*.rs
	cargo build --target x86_64-unknown-linux-musl --release --bin $(@F)
	touch $@

target/release/fs-writer: src/fs-writer/bin/*.rs
	cargo build --release --bin $(@F)

artifacts/bootstrap-rootfs.ext4: target/release/init
	mkdir -p mntdir
	if [ ! -f $@ ]; then truncate -s 1M $@ && mkfs.ext4 $@; fi
	sudo mount $@ mntdir
	# sudo cp -t mntdir artifacts/strace artifacts/busybox
	sudo strip target/x86_64-unknown-linux-musl/release/init
	sudo cp target/x86_64-unknown-linux-musl/release/init mntdir/init
	sudo mkdir -p mntdir/{dev,sys,proc,destination}
	sudo umount mntdir
	touch $@

output.ext4:
	truncate -s 350M output.ext4
	mkfs.ext4 output.ext4

run: target/release/fs-writer artifacts/bootstrap-rootfs.ext4 output.ext4
	./target/release/fs-writer --in-file disk.tar.gz --out-fs output.ext4 --pad-input-with-zeroes

test_artifacts: $(FS_ARTIFACTS)
	:
$(FS_ARTIFACTS):
	truncate -s 300M $@
	mkfs$(suffix $@) -q $@
	truncate -s 200K $@
