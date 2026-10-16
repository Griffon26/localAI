#!/bin/sh

echo "--- Mount filesystems ---"
mount -t proc proc /proc
mount -t sysfs sysfs /sys
# The kernel already mounts this
#mount -t devtmpfs none /dev
mkdir -p /dev/pts
mount -t devpts devpts /dev/pts
mount -t tmpfs -o size=1G tmpfs /tmp

echo "--- Setup overlayfs ---"
mkdir -p /tmp/upper /tmp/work
mount -t overlay overlay -o lowerdir=/home/sandboxuser,upperdir=/tmp/upper,workdir=/tmp/work /home/sandboxuser

echo "--- Mount repos ---"
mount /dev/vdb /mnt/repos

# Fix ownership so sandboxuser can write history files, git config, and code
chown -R sandboxuser:sandboxgroup /home/sandboxuser

echo "--- Set env vars ---"
# Configure terminal session
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin"
export HOME=/root
export TERM=xterm-256color

echo "--- Custom Script Init Booted Successfully ---"
echo "--- Oh My Pi Harness Ready ---"

getty -n  -l /sbin/autologin.sh ttyS0 115200 xterm-256color

echo "--- /bin/bash fallback ---"
/bin/bash
