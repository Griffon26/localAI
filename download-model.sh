#!/bin/bash

cd out

[ -f "selected_model.sh" ] || { echo "Run select_model.sh first" >&2; exit -1; }

source "selected_model.sh"

wget "${MODEL_URL}"

