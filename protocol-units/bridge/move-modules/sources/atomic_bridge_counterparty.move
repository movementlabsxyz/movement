module atomic_bridge::atomic_bridge_counterparty {
    use std::signer;
    use std::event;
    use std::vector;
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;
    use aptos_framework::aptos_hash::keccak256;
    use aptos_std::smart_table::{Self, SmartTable};
    use moveth::moveth;

    /// A mapping of bridge transfer IDs to their details
    struct BridgeTransferStore has key, store {
        pending_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        completed_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        aborted_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        bridge_transfer_assets_locked_events: EventHandle<BridgeTransferAssetsLockedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_cancelled_events: EventHandle<BridgeTransferCancelledEvent>,
    }

    struct BridgeTransferDetails has key, store {
        initiator: vector<u8>, // eth address
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
        signer_cap: account::SignerCapability,
    }

    #[event]
    /// An event triggered upon locking assets for a bridge transfer 
    struct BridgeTransferAssetsLockedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    #[event]
    /// An event triggered upon completing a bridge transfer
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    }

    #[event]
    /// An event triggered upon cancelling a bridge transfer
    struct BridgeTransferCancelledEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }
    
    entry fun init_module(resource: &signer) {

        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource, @origin_addr);

        let bridge_transfer_store = BridgeTransferStore {
            pending_transfers: smart_table::new(),
            completed_transfers: smart_table::new(),
            aborted_transfers: smart_table::new(),
            bridge_transfer_assets_locked_events: account::new_event_handle<BridgeTransferAssetsLockedEvent>(resource),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(resource),
            bridge_transfer_cancelled_events: account::new_event_handle<BridgeTransferCancelledEvent>(resource),
        };
        let bridge_config = BridgeConfig {
            moveth_minter: signer::address_of(resource),
            bridge_module_deployer: signer::address_of(resource),
            signer_cap: resource_signer_cap
        };
        move_to(resource, bridge_transfer_store);
        move_to(resource, bridge_config);
    }
    
    public entry fun lock_bridge_transfer_assets(
        caller: &signer,
        initiator: vector<u8>, //eth address
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        time_lock: u64,
        recipient: address,
        amount: u64
    ) acquires BridgeTransferStore {
        assert!(signer::address_of(caller) == @origin_addr, 1);
        let bridge_store = borrow_global_mut<BridgeTransferStore>(@resource_addr);
        let details = BridgeTransferDetails {
            recipient,
            initiator,
            amount,
            hash_lock,
            time_lock: timestamp::now_seconds() + time_lock
        };

        smart_table::add(&mut bridge_store.pending_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferAssetsLockedEvent {
                bridge_transfer_id,
                recipient,
                amount,
                hash_lock,
                time_lock,
            },
        );

    }

    #[view]
    public fun bridge_transfers(bridge_transfer_id : vector<u8>) : BridgeTransferDetails acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = store();
        if (aptos_std::smart_table::contains(&store.transfers, bridge_transfer_id)) {
            return *aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);
        } else {
            return BridgeTransfer {
                amount: 0,
                originator: @atomic_bridge,
                recipient: vector::empty<u8>(),
                hash_lock: vector::empty<u8>(),
                time_lock: 0,
                state: 0,
            };
        }
    }
    
    public fun complete_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig, {
        let config_address = borrow_global<BridgeConfig>(@resource_addr).bridge_module_deployer;
        let resource_signer = account::create_signer_with_capability(&borrow_global<BridgeConfig>(@resource_addr).signer_cap);
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
    
    public fun abort_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>
    ) acquires BridgeTransferStore, BridgeConfig {
        // check that the signer is the bridge_module_deployer
        assert!(signer::address_of(caller) == borrow_global<BridgeConfig>(signer::address_of(caller)).bridge_module_deployer, 1);
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(caller));
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);

        // Ensure the timelock has expired
        assert!(timestamp::now_seconds() > details.time_lock, 2);

        smart_table::add(&mut bridge_store.aborted_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferCancelledEvent {
                bridge_transfer_id,
            },
        );
    }
    
    #[test_only]
    public fun set_up_test(origin_account: &signer, resource_addr: &signer) {

        create_account_for_test(signer::address_of(origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(origin_account, vector::empty<u8>(), vector::empty<u8>());

        init_module(resource_addr);
    }

    #[test (origin_account = @origin_addr, resource = @resource_addr, aptos_framework = @0x1)]
    public entry fun test_set_up_test(origin_account: &signer, resource: signer, aptos_framework: signer) {
        set_up_test(origin_account, &resource);
    }

    use std::debug;
    use std::string::{String, utf8};
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::primary_fungible_store;

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, source_account = @source_account, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    fun test_complete_bridge_transfer(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
        source_account: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        set_up_test(origin_account, &resource_addr);

        timestamp::set_time_has_started_for_testing(&aptos_framework);
        moveth::init_for_test(moveth);
        let receiver_address = @0xdada;
        let initiator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();
        
        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); 
        let time_lock = 3600;
        let amount = 100;
        lock_bridge_transfer_assets(
            origin_account,
            initiator,
            bridge_transfer_id,
            hash_lock,
            time_lock,
            recipient,
            amount
        );
        //assert!(result, 1);
        // Verify that the transfer is stored in pending_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store.pending_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 2);
        assert!(transfer_details.initiator == initiator, 3);
        assert!(transfer_details.amount == amount, 5);
        assert!(transfer_details.hash_lock == hash_lock, 5);
        let pre_image = b"secret"; 
        let msg:vector<u8> = b"secret";
        debug::print(&utf8(msg));
        complete_bridge_transfer(
            client,
            bridge_transfer_id,
            pre_image, 
        );
        debug::print(&utf8(msg));
        // Verify that the transfer is stored in completed_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store. completed_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 1);
        assert!(transfer_details.amount == amount, 2);
        assert!(transfer_details.hash_lock == hash_lock, 3);
        assert!(transfer_details.initiator == initiator, 4);
    }
}
