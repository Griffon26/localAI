#!/bin/bash

[ -f "out/selected_model.sh" ] || { echo "Run select_model.sh first" >&2; exit -1; }

source "out/selected_model.sh"

[ -f "out/${MODEL_FILE}" ] || { echo "Run download_model.sh first" >&2; exit -1; }

llama-server \
  --jinja \
  -m "out/${MODEL_FILE}" \
  --host 172.16.0.1 \
  --port 9931 \
  ${MODEL_OPTIONS} \
  --flash-attn on

