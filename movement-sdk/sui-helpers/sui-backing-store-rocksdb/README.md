# `canonical-sui-backing-store`
A crate providing a RocksDB-based implementation of the Sui backing store.

- The original concept for this backing store was sourced from MystenLab's in memory store implementation: https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/simulacrum/src/store/in_mem_store.rs


## Challenges
- RocksDB relies on a native library librocksys. Versions of RocksDB in Sui, Aptos, and any other crate used within this workspace must remain in sync. 