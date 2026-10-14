#!/bin/bash

ARCH="$(uname -m)"
S3="https://s3.amazonaws.com/spec.ccfc.min"

# Find the latest CI artifact directory
CI_PREFIX=$(
  curl -fsSL "$S3?list-type=2&prefix=firecracker-ci/&delimiter=/" |
  grep -oP '(?<=<Prefix>)firecracker-ci/[0-9]{8}-[^/]+/(?=</Prefix>)' |
  sort |
  tail -1
)

# Find the newest kernel version for this architecture
KERNEL_KEY=$(
  curl -fsSL "$S3?list-type=2&prefix=${CI_PREFIX}${ARCH}/vmlinux-" |
  grep -oP "(?<=<Key>)${CI_PREFIX}${ARCH}/vmlinux-[0-9]+\.[0-9]+\.[0-9]{1,3}(?=</Key>)" |
  sort -V |
  tail -1
)

# Download it
test -n "$KERNEL_KEY" || {
  echo "No matching kernel artifact found"
  exit 1
}

wget -O vmlinux "$S3/$KERNEL_KEY"

file vmlinux
