#!/bin/bash
set -e

[ -d "out/rust" ] || { echo "Run build-vsock-sync.sh first" >&2; exit -1; }

echo "1. Compiling guest filesystem archive via podman build..."
podman build --output type=tar,dest=./out/rootfs.tar .

echo "2. Allocating empty disk image file..."
truncate -s 1G out/rootfs.ext4

echo "3. Unpacking rootfs.tar to ext4 filesystem..."
mkfs.ext4 -F -d out/rootfs.tar out/rootfs.ext4

echo "--------------------------------------------------------"
echo "Success! rootfs.ext4 created."
echo "--------------------------------------------------------"

