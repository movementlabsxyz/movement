# Transaction Pipe 
We have modified the [transaction_pipe.rs](./transaction_pipe.rs) from its original implementation to more directly use the Aptos mempool. This renders the `garbage_collected` sequence number mempool obsolete, but has a series of consequences for DoS and Gas Attacks. Reverting the feature set added in [High Sequence Number Gas DOS](https://github.com/movementlabsxyz/movement/pull/597) (which was closed under a separate PR).

## `SEQUENCER_NUMBER_TOO_OLD` and `SEQUENCER_NUMBER_TOO_NEW` Errors
The relevant code from the `CoreMempool` can be identified here: https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/mempool/src/core_mempool/mempool.rs#L99

The Aptos Mempool will only throw a `SEQUENCE_NUMBER_TOO_OLD` error if the sequence number is invalidated by the VM as too old. Aptos will remove sequence numbers which are too old from the mempool, as in theory gas has already been paid. This invalidation is lightweight and thus is considered DoS resistant, and since gas was already charged for the transaction, it is also sybil resistant.

Aptos will throw a `SEQUENCE_NUMBER_TOO_NEW` error if the sequence number is validated by the VM as too new. This is a more serious error, as it indicates that the transaction is not yet valid. `reject_transaction` does not remove the transaction from the mempool as the user must be required to pay gas for this transaction at some point. If it were not the case, then the user could load up the mempool with transactions that would never be executed. At the time of writing, it would seem this is still possible using the raw Aptos Mempool. 

Unfortunately, our asynchronous usage of the mempool may require us to reintroduce a tolerance as was introduced in #597. 
