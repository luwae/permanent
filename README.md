# Permanent: Persistent Memory and NVMe Non-Deterministic Tester

This is the source code of Permanent, a tool for automated crash consistency testing for file systems using persistent memory and NVMe.
It is a combination of [Vinter](https://os.itec.kit.edu/65_3814.php) [(source)](https://github.com/KIT-OSGroup/vinter) and [Revin](https://os.itec.kit.edu/97_3853.php).

## Setup

```sh
# Rust via rustup (see https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-unknown-linux-musl

# QEMU build dependencies
dnf install ninja-build glib2-devel pixman-devel
# get QEMU
wget https://download.qemu.org/qemu-8.0.4.tar.xz
tar xvJf qemu-8.0.4.tar.xz && mv qemu-8.0.4 qemu
# apply patch
pushd qemu && patch -p1 < ../qemu.patch && popd
# configure QEMU
mkdir qemu/build && pushd qemu/build && ../configure --target-list=x86_64-softmmu --enable-debug --enable-plugins && popd
# build QEMU
make -C qemu/build -j$(nproc)

# build permanent
./build.sh

```

## Executing a test case

 - create a working directory including `vm_config.yaml` and `test_config.yaml`
 - include `pmem_base.raw` or `nvme_base.raw` or both image files into the working directory, depending on the FS type. Usually these files are all zeroes with the respective size of the devices.
 - execute the pipeline stages. They are executed separately. Alternatively use `pipeline.sh {workdir}` which redirects the output of the pipeline stages into the working directory.

## License

Permanent is released under the MIT license, see `LICENSE` for details.
