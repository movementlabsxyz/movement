# `sui-block-executor`
The `sui-block-executor` is a block-based execution layer providing Move VM execution which features the Sui object runtime, Sui-like concurrency, Sui-like side effects, and the ability to integrate with Aptos.

## Overview
Essentially, the `sui-block-executor` performs the duty of...
1. Transforming [`Vec<SenderSignedData>`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-types/src/transaction.rs#L2019) into a [`Vec<VerifiedExecutableTransaction>`](https://github.com/MystenLabs/sui/blob/main/crates/sui-types/src/executable_transaction.rs#L55), and then, 
2. Executing each element in said vector in deterministic order via [`execute_transaction_to_effects`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/src/latest.rs#L79). 
3. `execute_transaction_to_effects` both returns the `TransactionEffects` and applies them to a temporary store within the `AuthorityStore` (which implements `BackingStore` via [required traits](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-types/src/storage/mod.rs#L489)). These effects must be committed--which is discussed further below. 

> Perhaps the most analogous path in the existing Sui source to what needs to be developed is the [`process_certificate` method from the `AuthorityStore`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1339). This is a good point of reference for many parts of this project, hence it is included in the reading material. 

Let's break `sui-block-executor` down.

### `Vec<SenderSignedData` to `Vec<VerifiedExecutableTransaction>`
A [`VerifiedExecutableTransaction`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-types/src/executable_transaction.rs#L14) is two important things:
    - `SenderSignedData` which constitutes the actual transactions in a programmable transaction block sent by the user.
    - `CertificateEnvelope` which certifies that a transaction belongs in a given epoch. 

Obtaining a `CertificateEnvelope` in an agnostic manner is part of the responsibility of the `sui-block-executor`. In the skeleton, you will see that there is a dependency on a `Box<dyn CerificateProvider>` which is responsible for providing the `CertificateEnvelope` for a given `SenderSignedData`.

That being said, in the proof concept phase, we will mainly rely on simply transforming `Vec<SenderSignedData>` into `Vec<VerifiedExecutableTransaction>` by certifying each `SenderSignedData` with a `CertificateEnvelope` indicating a system transaction. 

For the near-term goal of obtaining something more meaningful in Avalanche, we will likely want to consider transforming the signature of the data on the block itself. From the current structure--which would not have this data--we would likely use an additional backing store which maps `Vec<SenderSignedData>` to blocks with their completed information as opposed to just `Vec<SenderSignedData>`.

### The Dependencies of `execute_transaction_to_effects`
The Signature of [`execute_transaction_to_effects` is as follows](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/src/latest.rs#L79):
```rust
fn execute_transaction_to_effects(
    &self,
    store: &dyn BackingStore,
    protocol_config: &ProtocolConfig,
    metrics: Arc<LimitsMetrics>,
    enable_expensive_checks: bool,
    certificate_deny_set: &HashSet<TransactionDigest>,
    epoch_id: &EpochId,
    epoch_timestamp_ms: u64,
    input_objects: CheckedInputObjects,
    gas_coins: Vec<ObjectRef>,
    gas_status: SuiGasStatus,
    transaction_kind: TransactionKind,
    transaction_signer: SuiAddress,
    transaction_digest: TransactionDigest,
) -> (
    InnerTemporaryStore,
    TransactionEffects,
    Result<(), ExecutionError>,
)
```

- `&dyn BackingStore`: arguably the most important of the parameters. We essentially want to do this over RocksDB on our own. There may be several crates for this developed over time, but `canonical-sui-backing-store` is where we will start.
- `protocol_config`: this is a struct that pops up all over the Sui crate and was originally a big part of the cause for hesitancy in implementing the Sui block-executor at this level--owing to the presumed entanglement of consensus in parts of the object runtime specifically attributable to this config. However, it turns out that consensus parameters are not in fact used. Instead, what this is [used for at this level](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/v1/sui-adapter/src/programmable_transactions/execution.rs#L639) are things such as...
    - shared object deletion settings,
    - Move binary format settings,
    - Max package size.
- `metrics`: this one we can use [directly from Sui](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-types/src/metrics.rs#L9).
- `enable_expensive_checks`: this is a bool which is used to determine whether to perform expensive checks. We can ignore this for now and set to it whatever works.
- `certificate_deny_set`: this isn't actually used anywhere in Sui's source. We can ignore it and leave it empty.
- `epoch_id`: this is used to determine the epoch id of the transaction. We don't need a notion of epochs right now. But, you'll see that the executor has a `Box<dyn EpochProvider>` which is used to obtain the epoch id. We can ignore this for now and set it to whatever works.
- `epoch_timestamp_ms`: this is used to determine the epoch timestamp of the transaction. We don't need a notion of epochs right now. But, you'll see that the executor has a `Box<dyn EpochProvider>` which is used to obtain the epoch timestamp. We can ignore this for now and set it to whatever works.
- `input_objects` : this is slightly more involved and will probably be where we actually have to do some optimization after putting something naive together. The input objects can actually be derived from the [transaction](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L963). However, essentially, the reason `execute_transaction_to_effects` requires input objects is because the versions of the objects will have changed from when the transaction was submitted. The `AuthorityStore` handles this by [reading new input objects from its state](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L963). To help us separate concerns here, you'll see that the `sui-block-executor` has a `Box<dyn InputObjectProvider>` which is used to obtain the input objects. 
- `gas_coins`: this comes from the [transaction data itself](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1366).
- `gas_status`: this also can come from the [transaction data itself](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-transaction-checks/src/lib.rs#L50). it relies on a reference gas price and protocol config. We currently abstract this away with a `Box<dyn GasStatusProvider>`.
- `transaction_kind`: this comes from the [transaction data itself](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1366).
- `transaction_signer`: this comes from the [transaction data itself](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1366).
- `transaction_digest`: once we have the `VerifiedExecutableTransaction` we can [obtain the digest from it](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1363).


### Dealing with `TransactionEffects`
The [`AuthorityPerEpochStore`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority/authority_per_epoch_store.rs#L628) is really the engine of a lot of how Sui execution comes together for the Sui network. However, it unfortunately is also very entangled and asynchronous.

The effects are then asynchronously committed to the epoch store using the returned [`TransactionEffects`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1213) and later make their way into persistent state. This process involves a lot of interdependencies in Sui. However, we do not necessarily have to implement this directly. Take a look here to see for example how they are not used directly in the [`AuthorityStore`'s execution_driver](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/execution_driver.rs#L103). However, there's additional complexity here that is addressed below with which we are not yet sure whether we will need to deal.

To accomplish this, we must build a subsystem [similar to](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L1213). However, ours can be synchronous.

This is an ongoing area of research.

## Sui Object Runtime
`execute_transaction_to_effects` takes care of the Sui object runtime provided, so long as you are constructing the executor with a MoveVM similar to that produced by [`new_move_vm`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/src/latest.rs#L55C35-L55C35). In fact `new_move_vm` gives a good entry point for extending the function table, which should be enough for the appropriate integration.

## Sui-like Concurrency
We need to deterministically group transactions into execution groups that can be safely and maximally executed in parallel based on their sets of shared objects.

Obtaining shared objects can be accomplished via the [transaction data directly](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L963C78-L963C78). 

The simplest way to solve this problem is to brute-force through possible groupings in transaction order which is $O(n^2 * ||S||)$. However, practically the sizes of the sets will be much smaller than $||S||$ and the number of transactions in a block should be on the order of 100s.

```python
function find_parallel_groups(sets):
    parallel_groups = []  // List to hold groups of parallel sets
    for current_set in sets:
        added_to_group = False
        for group in parallel_groups:
            if current_set is disjoint with every set in group:
                add current_set to group
                added_to_group = True
                break
        if not added_to_group:
            create new group with current_set
            add this new group to parallel_groups
    return parallel_groups
```

> I believe you can do better with a trie algorithm. But, I haven't had time to verify. It would look something like this:
> 1. Insert a null node as the root in the trie.
> 2. Sort all the elements in the shared object sets.
> 3. For each set, insert the elements in the set into the trie.
> 4. Insert the transaction hash as if it were the last element in the set.
> 5. Run each separate subtree from the root in parallel, by doing an in-order traversal to obtain the transaction hash then executing the corresponding transaction. 
> The ordering of the elements in the sets ensures that intersecting sets will have the same path/subtree in the trie from the root.
> The complexity should be $O(n * ||S|| *log(||S||))$ I believe, which would be better in almost all cases because $||S||$ is likely to be smaller than $n$. However, I haven't had time to verify this.

A more optimistic alternative is to use something similar to Block-STM with reruns on collisions. However, this can be entertained at a later date.

## Sui-like Side Effects
`execute_transaction_to_effects` accomplishes a good deal of this. But, we still need to decide upon what to do about checkpoints and epochs. This can be tabled for now.

## Integration with Aptos
This is actually fairly straightforward on the Sui side. `new_move_vm` allows us to accept the Aptos function table.
