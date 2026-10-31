#!/bin/bash
set -eu

[ -d "out/rust" ] || { echo "Run build-vsock-sync.sh first" >&2; exit -1; }

LOCALTIME=$(readlink -f /etc/localtime)

case "$LOCALTIME" in
    */zoneinfo/*)
        HOST_TZ=${LOCALTIME#*/zoneinfo/}
        ;;
    *)
        echo "Cannot determine host IANA timezone from /etc/localtime" >&2
        exit 1
        ;;
esac
echo "Host timezone: $HOST_TZ"

echo "1. Compiling guest filesystem archive via podman build..."
podman build --build-arg HOST_TZ="$HOST_TZ" --output type=tar,dest=./out/rootfs.tar .

echo "2. Allocating empty disk image file..."
truncate -s 1G out/rootfs.ext4

echo "3. Unpacking rootfs.tar to ext4 filesystem..."
mkfs.ext4 -F -d out/rootfs.tar out/rootfs.ext4

echo "--------------------------------------------------------"
echo "Success! rootfs.ext4 created."
echo "--------------------------------------------------------"

