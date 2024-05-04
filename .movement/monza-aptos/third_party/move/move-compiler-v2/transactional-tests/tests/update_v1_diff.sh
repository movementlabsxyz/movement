#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/bash
set -e
BASE=$(git rev-parse --show-toplevel)
echo "Assuming repo root at $BASE"
cd $BASE/third_party/move
cargo run -p testdiff -- -m -e > $BASE/third_party/move/move-compiler-v2/transactional-tests/tests/v1.matched
cargo run -p testdiff -- -u -e > $BASE/third_party/move/move-compiler-v2/transactional-tests/tests/v1.unmatched
