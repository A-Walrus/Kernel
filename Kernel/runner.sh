#!/bin/bash
cp ../image.img ../disk.img
cd simple_boot
cargo build --release
cd ..
target/x86_64-unknown-linux-gnu/release/simple_boot $1
