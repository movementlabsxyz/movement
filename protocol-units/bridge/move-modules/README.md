# Atomic Bridge Move Modules 


## `moveth.move`
This module offers a reference implementation of a managed stablecoin with the following functionalities:
1. Upgradable smart contract. The module can be upgraded to update existing functionalities or add new ones.
2. Minting and burning of stablecoins. The module allows users to mint and burn stablecoins. Minter role is required to mint or burn
3. Denylisting of accounts. The module allows the owner to denylist (freeze) and undenylist accounts.
denylist accounts cannot transfer or get minted more.
4. Pausing and unpausing of the contract. The owner can pause the contract to stop all mint/burn/transfer and unpause it to resume.

## Running tests
aptos move test

## Deploy flow with resource account

To publish this package under a resource account using [Movement CLI](https://docs.movementnetwork.xyz/devs/movementcli) 

1. Run `movement init` to create an origin account address. 

2. Run:

```
movement move create-resource-account-and-publish-package --address-name resource_addr --seed <any-string-as-a-seed>
```

If successful, you'll get the following prompt: 

```
Do you want to publish this package under the resource account's address 0xdc04f3645b836fd1c0c2bd168b186cb98d70206122069d257871856e7a1834a7? [yes/no]
```

However, because dummy values `0xcafe` for origin address and `0xc3bb...` for resource address are included for testing in `Move.toml` already, you'll likely get this instead:

```
{
  "Error": "Unexpected error: Unable to resolve packages for package 'bridge-modules': Unable to resolve named address 'resource_addr' in package 'bridge-modules' when resolving dependencies: Attempted to assign a different value '0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5' to an a already-assigned named address '0x9a781a5a11e364a7a67d874071737d730d14a43847c5025066f8cd4887212d4'"
}
```
with your generated resource account address (instead of `0x9a78...`) based on your choice of seed value.

3. Copy the resource account address, then input "no".

4. In `Move.toml` replace the value of
- `resource_addr`, `moveth`, and `atomic_bridge` with the resource account address in the prompt, and
- `origin_addr` with the account address generated in `movement init`.

5. Run `movement move create-resource-account-and-publish-package` again with the same resource account name and the same seed.

This time after inputting "yes" to the first prompt, you should get

```
Do you want to submit a transaction for a range of [1592100 - 2388100] Octas at a gas unit price of 100 Octas? [yes/no]
```

Input "yes" and the package will be published under the resource account.

## How does the resource account pattern work, for minting?

The resource account pattern is implemented for the sake of allowing the atomic bridge to autonomously mint MovETH in `complete_bridge_transfer`. There is no private key associated with a resource account, so there is no possibility of any bad actors being able to control the mint process; rather the rules defined in `complete_bridge_transfer` control access.

The following changes were made in PR [#362](https://github.com/movementlabsxyz/movement/pull/362) to enable the resource account pattern:

In `atomic_bridge_counterparty.move`

- ## Replace the `init_for_test` function with a `set_up_test` function and modify `init_module`:

```
    #[test_only]
    public fun set_up_test(origin_account: signer, resource: &signer) {

        create_account_for_test(signer::address_of(&origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(&origin_account, vector::empty<u8>(), vector::empty<u8>());

        init_module(resource);
    }
```

This setup function
1. creates an account from the origin account address,
2. creates a resource account deterministically from the origin account,
3. Calls `init_module`:

```
    entry fun init_module(resource: &signer) {

        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource, @0xcafe);

        let bridge_transfer_store = BridgeTransferStore {
            pending_transfers: smart_table::new(),
            completed_transfers: smart_table::new(),
            aborted_transfers: smart_table::new(),
        };
        let bridge_config = BridgeConfig {
            moveth_minter: signer::address_of(resource),
            bridge_module_deployer: signer::address_of(resource),
            signer_cap: resource_signer_cap
        };
        move_to(resource, bridge_transfer_store);
        move_to(resource, bridge_config);
    }
```
1. First the resource account signer cap is retrieved. Notice the origin address must be hard-coded in as a param. (See `test_set_up_test` below for explanation of the relationship between origin address and resource address.)
2. The `BridgeConfig` which was modified to include a `signer_cap` field, is moved to the resource account, along with the `BridgeTransferStore`. 

- ## Add `test_set_up_test`:

```
    #[test (origin_account = @0xcafe, resource = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, aptos_framework = @0x1)]
    public entry fun test_set_up_test(origin_account: signer, resource: signer, aptos_framework: signer) {
        set_up_test(origin_account, &resource);
    }
```

To explain the choice of addresses, we can look at the resource account creation:

```
        resource_account::create_resource_account(&origin_account, vector::empty<u8>(), vector::empty<u8>());
```

The `0xcafe` origin address deterministically generates the resource address `0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5`.

`resource_account::create_resource_account` calls `account::create_resource_account` which in turn calls `account::create_resource_address`:

```
    /// This is a helper function to compute resource addresses. Computation of the address
    /// involves the use of a cryptographic hash operation and should be use thoughtfully.
    public fun create_resource_address(source: &address, seed: vector<u8>): address {
        let bytes = bcs::to_bytes(source);
        vector::append(&mut bytes, seed);
        vector::push_back(&mut bytes, DERIVE_RESOURCE_ACCOUNT_SCHEME);
        from_bcs::to_address(hash::sha3_256(bytes))
    }
```  

- ## Modify `complete_bridge_transfer`:

```
    public fun complete_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig, {
        let config_address = borrow_global<BridgeConfig>(@0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5).bridge_module_deployer;
        let resource_signer = account::create_signer_with_capability(&borrow_global<BridgeConfig>(@0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5).signer_cap);
        let bridge_store = borrow_global_mut<BridgeTransferStore>(config_address);
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);

        let computed_hash = keccak256(pre_image);
        assert!(computed_hash == details.hash_lock, 2);

        moveth::mint(&resource_signer, details.recipient, details.amount);

        smart_table::add(&mut bridge_store.completed_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferCompletedEvent {
                bridge_transfer_id,
                pre_image,
            },
        );
    }
```

Instead of adding the caller as minter, `resource_signer` is created by borrowing the `signer_cap` from the `BridgeConfig`. Then `resource_signer` ref is passed in as signer for `moveth::mint`.

This works because in the `moveth` module, the `init_module` function was modified to include the resource address in the minters list:

```
      let minters = vector::empty<address>();
        vector::push_back(&mut minters, @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5);
```

Avoiding the need to add and then remove caller as minter, and simply having the resource signer facilitate minting, appears to be more efficient.
