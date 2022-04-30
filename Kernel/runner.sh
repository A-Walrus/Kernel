#!/bin/bash
cd ../Userspace
cargo build --release
../update_image.sh
cd ../Kernel/simple_boot
cargo build --release
cd ..
target/x86_64-unknown-linux-gnu/release/simple_boot $1
