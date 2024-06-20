#!/bin/bash
set -e
docker build -t test-celestia-appd -f ./Dockerfile . && docker run -it --rm -v "$(pwd)":/mnt -p 26658:26658 test-celestia-appd /bin/sh
