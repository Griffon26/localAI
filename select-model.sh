#!/usr/bin/env bash
set -euo pipefail

mkdir -p out

OUTPUT_FILE="out/selected_model.sh"

declare -A models

# Populate the dictionary
models["Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL"]='
  export MODEL_FILE="Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL.gguf"
  export MODEL_URL="https://huggingface.co/unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF/resolve/main/Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL.gguf"
  export MODEL_OPTIONS="-c 65536 -ngl 34 -ctk q8_0 -ctv q8_0"
'
models["qwen2.5-coder-14b-instruct-q4_k_m"]='
  export MODEL_FILE="qwen2.5-coder-14b-instruct-q4_k_m.gguf"
  export MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-Coder-14B-Instruct-GGUF/resolve/main/qwen2.5-coder-14b-instruct-q4_k_m.gguf"
  export MODEL_CONTEXT="32768"
  export MODEL_OPTIONS="-c 32768 -ngl 99"
'

# Extract keys and sort them alphabetically into the options array
mapfile -t options < <(printf '%s\n' "${!models[@]}" | sort)

echo "=========================================="
echo "       Select a model to use              "
echo "=========================================="

PS3="Please enter your choice (1-${#options[@]}): "
select opt in "${options[@]}"; do
    if [[ -n "${opt:-}" ]]; then
        echo "Selected: ${opt}"
        
        cat << EOF > "$OUTPUT_FILE"
# Generated Environment Variables for: ${opt}
${models["$opt"]}
EOF
        break
    else
        echo "Invalid selection. Please choose a valid number."
    fi
done

echo "Environment file successfully generated at: ./$OUTPUT_FILE"
