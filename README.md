# Prerequisites

- installed packages:
  - e2fsprogs (built with libarchive support)
  - podman
  - firecracker
  - rustup
- a tap network interface called tap0 with address 172.16.0.1 and no default gateway
- a repos.ext4 disk image for storing modifiable code, can be created like this:
  - truncate -s 500M repos.ext4
  - mkfs.ext4 repos.ext4
  
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

