export LITELLM_LOCAL_MODEL_COST_MAP=True
#export PI_DEBUG_STARTUP=1
#export PI_TIMING=1

if [ -t 0 ]; then
  resize
  cd /mnt/repos
  # /bin/bash  # enable to debug the environment before start omp
  exec omp
fi

