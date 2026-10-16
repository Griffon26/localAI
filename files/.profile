export LITELLM_LOCAL_MODEL_COST_MAP=True
export OLLAMA_BASE_URL="http://172.16.0.1:11434"
export PI_DEBUG_STARTUP=1
export PI_TIMING=1

if [ -t 0 ]; then
  resize
  /bin/bash
  exec omp
fi

