#!/bin/bash

set -euo pipefail

[ -f "out/vmlinux" ] || { echo "Run download-kernel.sh first" >&2; exit -1; }
[ -f "out/rootfs.ext4" ] || { echo "Run build-rootfs.sh first" >&2; exit -1; }

# Copy workspace contents into workspace filesystem image
tar cf out/workspace.tar -C workspace .
truncate -s 1G out/workspace.ext4
mkfs.ext4 -F -d out/workspace.tar out/workspace.ext4

# Start firecracker
SOCKETDIR=out/firecracker-sockets
mkdir -p "${SOCKETDIR}"
chmod go-rwx "${SOCKETDIR}"
firecracker --api-sock "${SOCKETDIR}/api.sock" --config-file vm_config.json
rm -rf "${SOCKETDIR}"

