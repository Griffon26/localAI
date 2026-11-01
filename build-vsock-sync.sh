#!/bin/bash
set -euo pipefail

cd vsock-sync

# Build for the host
cargo build --release

# Build for the guest
cargo build --release --target x86_64-unknown-linux-musl

