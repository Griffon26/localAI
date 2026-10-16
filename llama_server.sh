#!/bin/bash

llama-server \
  --jinja \
  -m qwen2.5-coder-14b-instruct-q4_k_m.gguf \
  --host 172.16.0.1 \
  --port 9931 \
  -c 32768 \
  -ngl 99 \
  --flash-attn on

