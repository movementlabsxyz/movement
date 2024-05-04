#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/sh
# Copyright (c) The Diem Core Contributors
# Copyright (c) The Move Contributors
# SPDX-License-Identifier: Apache-2.0


export BASE="$(git rev-parse --show-toplevel)/language/move-prover/lab/data/quantifiers"

jupyter lab ${BASE}/notebook.ipynb
