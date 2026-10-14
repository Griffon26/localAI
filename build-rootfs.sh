#!/bin/bash
set -e

echo "1. Compiling guest filesystem archive via podman build..."
podman build --output type=tar,dest=./rootfs.tar .

echo "2. Allocating empty disk image file..."
truncate -s 1G rootfs.ext4

echo "3. Unpacking rootfs.tar to ext4 filesystem..."
mkfs.ext4 -F -d rootfs.tar rootfs.ext4

echo "--------------------------------------------------------"
echo "Success! rootfs.ext4 created."
echo "--------------------------------------------------------"

