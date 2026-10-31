#!/bin/bash
echo "Allocating 1G empty ext4 disk image file..."
truncate -s 1G out/workspace.ext4
mkfs.ext4 out/workspace.ext4

