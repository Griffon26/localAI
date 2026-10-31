#!/bin/bash

[ -f "out/vmlinux" ] || { echo "Run download-kernel.sh first" >&2; exit -1; }
[ -f "out/rootfs.ext4" ] || { echo "Run build-rootfs.sh first" >&2; exit -1; }

SOCKETDIR=out/firecracker-sockets

mkdir -p "${SOCKETDIR}"
chmod go-rwx "${SOCKETDIR}"
firecracker --api-sock "${SOCKETDIR}/api.sock" --config-file vm_config.json
rm -rf "${SOCKETDIR}"

