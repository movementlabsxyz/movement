#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/sh
# Copyright (c) The Diem Core Contributors
# Copyright (c) The Move Contributors
# SPDX-License-Identifier: Apache-2.0

FUN_RESULTS="opaque.fun_data ignore_internal_opaque.fun_data ignore_opaque.fun_data"
MOD_RESULTS="opaque.mod_data ignore_internal_opaque.mod_data ignore_opaque.mod_data"

# Plot per function
cargo run -q --release -p prover-lab -- \
    plot --out fun_by_fun.svg --sort ${FUN_RESULTS}

# Plot per module
cargo run -q --release -p prover-lab -- \
    plot --out mod_by_mod.svg --sort ${MOD_RESULTS}
