# `object_version` Providers
Sui execution relies on assigning sequence numbers to shared objects and shared object with sequence numbers to transactions. This is how it keeps determinism when executing. 

These are tracked in the tables shown [here]
(https://github.com/MystenLabs/sui/blob/6ec723bcbdc4c36358d444cbfcd88ae1378761a5/crates/sui-core/src/authority/authority_per_epoch_store.rs#L301).

We would like to implement similar functionality to give ourselves valid sequence numbers for shared objects and transactions that will operate appropriately under concurrency.

To this, we would like to implement the below...

```rust
async fn assign_shared_object_versions(&self, transactions : VerifiedExecutableExecutionGroups) -> Result<VerifiedExecutableExecutionGroups, anyhow::Error> {
    unimplemented!();
}
```

This method takes one of our blocks that has been grouped for parallelism, `VerifiedExecutableExecutionGroups`, and assigns the sequence numbers and transactions in the persisted store according to how that group would be executed. 

That is to say, if one of the groups acts on a particular object in three transactions, the sequence numbers assigned to those versions of the object should be 1, 2, and 3--assuming the sequence number was originally 0. The object versions with those sequence numbers should also be assigned to the transactions. Those numbers are persisted and the next time a block executes, the sequence numbers should be 4, 5, and so on.

We would like to add common helper functions for maintainability and flexibility. Please adjust the trait as you see fit.