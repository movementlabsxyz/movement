# `sov-aptos` module

The sov-aptos module provides compatibility with the aptos.

The module `CallMessage` contains `rlp` encoded Ethereum transaction, which is validated & executed immediately after being dispatched from the DA. Once all transactions from the DA slot have been processed, they are grouped into an `Ethereum` block. Users can access information such as receipts, blocks, transactions, and more through standard Ethereum endpoints.

## Note to developers (hooks.rs)

WARNING: `prevrandao` value is predictable up to `DEFERRED_SLOTS_COUNT` in advance,
Users should follow the same best practice that they would on Ethereum and use future randomness.
See: `<https://eips.ethereum.org/EIPS/eip-4399#tips-for-application-developers>`