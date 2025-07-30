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

|                                             | MVT        | APTOS          | MIGATED Network |
| APTOS_STD_CHAIN_ID_NATIVES                  | ENABLE    | DISABLE|  |
| PERIODICAL_REWARD_RATE_DECREASE             | DISABLE  | ENABLE   |          
| PARTIAL_GOVERNANCE_VOTING                   | DISABLE  | ENABLE|  |
| DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING   | DISABLE  | ENABLE|  |
| VM_BINARY_FORMAT_V7                         | DISABLE  | ENABLE         |   DISABLE |
| KEYLESS_BUT_ZKLESS_ACCOUNTS                 | ENABLE    | DISABLE
| REMOVE_DETAILED_ERROR_FROM_HASH             | ENABLE    | DISABLE         |   DISABLE |
| KEYLESS_ACCOUNTS_WITH_PASSKEYS              | ENABLE    | DISABLE          |  DISABLE |
| NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE        | DISABLE  | ENABLE|  |
| CONCURRENT_FUNGIBLE_BALANCE                 | ENABLE    | DISABLE|  |
| DISALLOW_USER_NATIVES                       | DISABLE  | ENABLE |  |
| ALLOW_SERIALIZED_SCRIPT_ARGS                | DISABLE  | ENABLE |  |
| ENABLE_ENUM_TYPES                           | DISABLE  | ENABLE |  |
| REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT         | DISABLE  | ENABLE |  |
| FEDERATED_KEYLESS                           | DISABLE  | ENABLE |  |
| TRANSACTION_SIMULATION_ENHANCEMENT          | DISABLE  | ENABLE |  |
| COLLECTION_OWNER                            | DISABLE  | ENABLE | |
| NATIVE_MEMORY_OPERATIONS                    | DISABLE  | ENABLE | |
| ENABLE_LOADER_V2                            | DISABLE  | ENABLE | |
| DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES     | DISABLE  | ENABLE | |
| NEW_ACCOUNTS_DEFAULT_TO_FA_STORE            | DISABLE  | ENABLE | |
| DEFAULT_ACCOUNT_RESOURCE                    | DISABLE  | ENABLE | |

The feature that are the same but must be changed are:
|                                             | MVT        | APTOS          | MIGATED Network |
| KEYLESS_ACCOUNTS                            | ENABLE   | ENABLE              | DISABLE |