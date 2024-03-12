use ethers::prelude::*;

abigen!(Settlement, "$CARGO_MANIFEST_DIR/out/Settlement.sol/Settlement.json");
abigen!(Counter, "$CARGO_MANIFEST_DIR/out/Counter.sol/Counter.json");