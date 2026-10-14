# Prerequisites

- installed packages:
  - e2fsprogs (built with libarchive support)
  - podman
  - firecracker
- a tap network interface called tap0 with address 172.16.0.1 and no default gateway
- a repos.ext4 disk image for storing modifiable code, can be created like this:
  - truncate -s 500M repos.ext4
  - mkfs.ext4 repos.ext4
  
# Quickstart

- ./build-rootfs.sh
- ./download-kernel.sh
- ./start-firecracker.sh

