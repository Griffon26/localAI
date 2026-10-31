# Prerequisites

- installed packages:
  - e2fsprogs (built with libarchive support)
  - podman
  - firecracker
  - rustup
- a tap network interface called tap0 with address 172.16.0.1 and no default gateway
  
# Quickstart

- set up a rust toolchain for your local user:
  - rustup-init
  - rustup toolchain install stable
  - rustup default stable
  - rustup target add x86_64-unknown-linux-musl
- ./build-vsock-sync.sh
- ./build-rootfs.sh
- ./download-model.sh
- ./download-kernel.sh
- ./start-firecracker.sh

