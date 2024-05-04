#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/bash

# cd to repo root
cd "$(git rev-parse --show-toplevel)"

pnpm install
pnpm test docker/__tests__