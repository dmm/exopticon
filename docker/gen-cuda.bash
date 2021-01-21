#!/bin/bash

export CUDA_DEVS=$(find /dev -maxdepth 1 -name 'nvidia*' | xargs -I{} echo '      - {}\n')
export CUDA_LIBS=$(find /usr/lib/x86_64-linux-gnu/{libnvcuvid,libcuda,libnvidia}* -maxdepth 1 -not -type d | xargs -I{} echo '      - {}:{}:ro\n')


envsubst < docker-compose.cuda.yml
