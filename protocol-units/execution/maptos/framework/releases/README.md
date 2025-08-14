# `framework/releases`
We use this directory to track framework releases and migrations. The path of migrations described herein is supported, however, other migrations may be viable for your network. 

- Each crate herein should export a [`ReleaseBundle](https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/aptos-move/framework/src/release_bundle.rs#L16) s.t. it can be used with 
    - `encode_genesis_change_set` for the genesis transaction;
    - framework upgrades.
- The [`latest`](./latest/) directory should contain the intended new framework release. It should always re-export a named release crate as `pub use <release_crate>::*`.
- The [`parent`](./parent/) directory should contain the framework release the preceded the current latest release. It should always re-export a named release crate as `pub use <release_crate>::*`.
- When making working on a new framework release, you should perform the following:
    1. Ensure the current [`latest`](./latest/) has a test that verifies the migration from the previous release by the backing names.
    2. Move the current [`latest`](./latest/) release to the [`parent`](./parent/) directory. The release that was previously the latest should now be presumed live on the network.
    3. Create a new directory appropriately named for the new release.
    4. Implement the new release in the new directory.
    5. Write a migration test for it against the named release which is now in the [`parent`](./parent/) directory.
    6. Update the [`latest`](./latest/) directory to re-export the new release.
- The [`head`](./head/) crate is a special case that is used to track the release that is on the current branch head. It can be used for intermediately testing changes that are not yet ready to be published.

# Feature migration

The feature comparission between MVT Network and Aptos Network gives these features that are different.

| ID | Title                                      | MVT     | APTOS   | MIGRATED Network |
|----|--------------------------------------------|---------|---------|------------------|
|  4 | APTOS_STD_CHAIN_ID_NATIVES                 | ENABLE  | DISABLE | DISABLE          |
|  6 | COLLECT_AND_DISTRIBUTE_GAS_FEES            | DISABLE | DISABLE | ENABLE           |
| 16 | PERIODICAL_REWARD_RATE_DECREASE            | DISABLE | ENABLE  | DISABLE          |  
| 17 | PARTIAL_GOVERNANCE_VOTING                  | DISABLE | ENABLE  | ENABLE           |
| 21 | DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING  | DISABLE | ENABLE  | ENABLE           |
| 40 | VM_BINARY_FORMAT_V7                        | DISABLE | ENABLE  | DISABLE          |
| 46 | KEYLESS_ACCOUNTS                           | ENABLE  | ENABLE  | DISABLE          |
| 47 | KEYLESS_BUT_ZKLESS_ACCOUNTS                | ENABLE  | DISABLE | DISABLE          |
| 48 | REMOVE_DETAILED_ERROR_FROM_HASH            | ENABLE  | DISABLE | DISABLE          |
| 54 | KEYLESS_ACCOUNTS_WITH_PASSKEYS             | ENABLE  | DISABLE | DISABLE          |
| 64 | NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE       | DISABLE | ENABLE  | ENABLE           |
| 67 | CONCURRENT_FUNGIBLE_BALANCE                | ENABLE  | DISABLE | DISABLE          |
| 71 | DISALLOW_USER_NATIVES                      | DISABLE | ENABLE  | ENABLE           |
| 72 | ALLOW_SERIALIZED_SCRIPT_ARGS               | DISABLE | ENABLE  | ENABLE           |
| 74 | ENABLE_ENUM_TYPES                          | DISABLE | ENABLE  | ENABLE           |
| 76 | REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT        | DISABLE | ENABLE  | ENABLE           |
| 77 | FEDERATED_KEYLESS                          | DISABLE | ENABLE  | ENABLE           |
| 78 | TRANSACTION_SIMULATION_ENHANCEMENT         | DISABLE | ENABLE  | ENABLE           |
| 79 | COLLECTION_OWNER                           | DISABLE | ENABLE  | ENABLE           |
| 80 | NATIVE_MEMORY_OPERATIONS                   | DISABLE | ENABLE  | ENABLE           |
| 81 | ENABLE_LOADER_V2                           | DISABLE | ENABLE  | ENABLE           |
| 82 | DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES    | DISABLE | ENABLE  | ENABLE           |
| 90 | NEW_ACCOUNTS_DEFAULT_TO_FA_STORE           | DISABLE | ENABLE  | ENABLE           |
| 91 | DEFAULT_ACCOUNT_RESOURCE                   | DISABLE | ENABLE  | ENABLE           |
| XX | GOVERNED_GAS_POOL                          | ENABLE  | ENABLE  | DISABLE          |

## Feature description
### 4 APTOS_STD_CHAIN_ID_NATIVES
Activate this function `native public fun get(): u8;` which allow to access the chain Id inside Move code.


### 6 PERIODICAL_REWARD_RATE_DECREASE
Enables scheduled reductions in validator/staker reward rates over epochs.

### 17 PARTIAL_GOVERNANCE_VOTING
Changes how governance proposals are resolved when not all validators vote

When this feature is disabled (default):
 - A proposal only passes if quorum is reached and a majority of the total voting power approves.
 - If not enough validators vote, the proposal fails by default.

When this feature is enabled:
 - The proposal outcome only considers the validators who actually voted.
 - Abstention is no longer equivalent to a "no" vote.


### 21 DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING
Allows partial vote counting for delegated stake within validator delegation pools

This feature changes how votes are counted within delegation pools, especially when:
 - Delegators do not cast votes
 - A validator or part of the pool abstains

With this feature enabled:
 - Only the delegated votes actually cast are used in calculating the outcome
 - It becomes possible for a subset of the delegation pool to pass/fail proposals

Without this feature, if a validator has 100 delegated tokens but only 30 are used to vote, the system assumes the other 70 abstained — and still counts them against quorum or majority thresholds


### 64 NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE
Changes the default storage model for APT coins in new accounts.
When NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE is enabled:

All new accounts created after feature activation will:
 - Use FA-based storage for their APT coins by default
 - No longer use CoinStore<AptosCoin> unless explicitly created that way

This enables faster scaling of account creation and APT payments at the protocol level.

Without this feature :
 - New accounts use CoinStore-based APT storage
 - Less concurrency for transfers
 - Less optimized for high-throughput workloads

### 67 CONCURRENT_FUNGIBLE_BALANCE
Enables a new implementation of CoinStore optimized for concurrency and performance

When CONCURRENT_FUNGIBLE_BALANCE is enabled, Aptos switches the underlying CoinStore implementation to a new model using:
 - Aggregator-backed CoinStore
 - Native Rust implementation optimized for:
  - Parallelism in the VM
  - Reduced gas costs for coin transfers
  - Concurrent writes to many CoinStores

### 71 DISALLOW_USER_NATIVES
`DISALLOW_USER_NATIVES` is a VM feature flag that forbids non-framework modules from defining Move “native” items (i.e., native fun and native struct).
When this flag is enabled, the Aptos VM will reject publishing or upgrading any module that contains user-defined natives unless it belongs to the core code addresses (e.g., 0x1 AptosFramework / AptosStd / MoveStdlib).

### 72 ALLOW_SERIALIZED_SCRIPT_ARGS
The Aptos feature flag ALLOW_SERIALIZED_SCRIPT_ARGS controls whether Move script/function arguments can be passed in serialized (BCS) form instead of structured command-line input.

Use Cases
 - Protocol governance automation: Tools like aptos-governance can submit exact arguments on-chain in BCS format
 - Cross-language SDKs: Rust, Python, JavaScript SDKs can encode arguments once and submit them to the chain without decoding
 - Batch transactions or replay systems can record + replay exact inputs

### 74 ENABLE_ENUM_TYPES
Enables the enum type system in Move (akin to Rust-style enums or sum types)

### 76 REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT
Prevent scripts from using unstable VM bytecodes (e.g., Move 2 experimental opcodes).

### 77 FEDERATED_KEYLESS
Enables federated (off-chain) authentication and signing for special keyless Aptos accounts

Keyless accounts:
 - Don’t store a public key in the account
 - Instead, rely on off-chain verifiable authentication protocols (e.g., WebAuthn, federated ID systems)
 - Authentication and signature checks are delegated to a federated service (trusted by the network)

### 78 TRANSACTION_SIMULATION_ENHANCEMENT
Improves consistency, safety, and reliability of transaction simulations

### 79 COLLECTION_OWNER
Enables collection ownership logic for NFTs (Token v2), giving collection creators ownership rights and management capabilities

### 80 NATIVE_MEMORY_OPERATIONS
Enables native (Rust-implemented) Move functions for low-level memory access or manipulation.
It enable these functions:

```
native public fun memcpy(dst: &mut vector<u8>, src: &vector<u8>, len: u64);
native public fun memcmp(a: &vector<u8>, b: &vector<u8>): bool;
```

### ENABLE_LOADER_V2
Activates Loader v2, a new Move module loading engine with improved design and performance

### 82 DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES
Prevent modules from calling move_to or publish_module during their own initialization (init_module)

### 90 NEW_ACCOUNTS_DEFAULT_TO_FA_STORE
Makes all newly created accounts default to using FA (aggregator-based) CoinStores for any fungible token, not just APT

### 91 DEFAULT_ACCOUNT_RESOURCE
Enables the use of the new DefaultAccount resource layout for newly created accounts

### GOVERNED_GAS_POOL
Movement feature removed.

## Proposition
From my understanding of the feature I propose theses changes:

### Features that can be enabled without risk

Note Features that have not been enabled are not

[x] CONCURRENT_FUNGIBLE_BALANCE
ALLOW_SERIALIZED_SCRIPT_ARGS 
REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT
TRANSACTION_SIMULATION_ENHANCEMENT
COLLECTION_OWNER
NATIVE_MEMORY_OPERATIONS
ENABLE_LOADER_V2
DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES
DISALLOW_USER_NATIVES


