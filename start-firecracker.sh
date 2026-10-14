#!/bin/bash
rm -f firecracker.sock
firecracker --api-sock ./firecracker.sock --config-file vm_config.json
rm -f firecracker.sock

