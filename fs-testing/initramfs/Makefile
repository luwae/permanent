
INITRAMFS_DEPS := \
	create-initramfs.sh \
	busybox \
	../../permanent_plugin/checkpoint \
	../fs-dump/target/x86_64-unknown-linux-musl/release/fs-dump

default: initramfs.cpio.gz
.PHONY: default

initramfs.cpio.gz: $(INITRAMFS_DEPS)
	./create-initramfs.sh > $@
	
initramfs_mainline.cpio.gz: $(INITRAMFS_DEPS)
	rm -rf initramfs_mainline
	mkdir -p initramfs_mainline/{bin,lib}
	ln -s lib initramfs_mainline/lib64
	./copy-binary.sh `which mkfs.ext4` initramfs_mainline
	./copy-binary.sh `which fsck.ext4` initramfs_mainline
	./copy-binary.sh `which mkfs.xfs` initramfs_mainline
	./copy-binary.sh `which xfs_repair` initramfs_mainline
	./create-initramfs.sh initramfs_mainline > $@

initramfs_zilpmem.cpio.gz: $(INITRAMFS_DEPS) ../zil-pmem/initramfs_zfs
	./create-initramfs.sh ../zil-pmem/initramfs_zfs > $@

busybox:
	curl -o $@ https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox
	chmod +x $@
