#!/bin/bash

cd out

[ -f "selected-model.sh" ] || { echo "Run select-model.sh first" >&2; exit -1; }

source "selected-model.sh"

wget "${MODEL_URL}"

