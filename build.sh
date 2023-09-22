#!/usr/bin/env bash

set -eux

cargo build --release

# build fs-dump
(cd fs-testing/fs-dump && cargo build --release --target=x86_64-unknown-linux-musl)

# build checkpoint
(cd permanent_plugin && ./build_checkpoint.sh)

# build initramfs
make -C fs-testing/initramfs
