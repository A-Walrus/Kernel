#!/bin/bash
cd simple_boot
cargo build
cd ..
target/x86_64-unknown-linux-gnu/debug/simple_boot $1