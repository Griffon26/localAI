#!/bin/bash

set -euo pipefail

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
chown sandboxuser:sandboxgroup /tmp/upper /tmp/work
mount -t overlay overlay -o lowerdir=/home/sandboxuser,upperdir=/tmp/upper,workdir=/tmp/work /home/sandboxuser

echo "--- Mount workspace ---"
mount /dev/vdb /mnt/workspace
chown -R sandboxuser:sandboxgroup /mnt/workspace

echo "--- Set env vars ---"
# Configure terminal session
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin"
export HOME=/root
export TERM=xterm-256color

echo "--- Getting context size from llama.cpp props endpoint ---"
CONTEXT_SIZE=$(curl -s http://172.16.0.1:9931/props | grep -o '"n_ctx":[^,}]*'|cut -d':' -f2)
echo "Context size: ${CONTEXT_SIZE}"

echo "--- Filling in context size in models.yml template ---"
YAML_DIR="/home/sandboxuser/.omp/agent"
cat "${YAML_DIR}/models.yml.template" | sed -e "s/contextWindow: NNNN/contextWindow: ${CONTEXT_SIZE}/" > "${YAML_DIR}/models.yml"
chown sandboxuser:sandboxgroup "${YAML_DIR}/models.yml"

echo "--- Custom Script Init Booted Successfully ---"
echo "--- Oh My Pi Harness Ready ---"

getty -n  -l /sbin/autologin.sh ttyS0 115200 xterm-256color

echo "--- /bin/bash fallback ---"
/bin/bash
