module atomic_bridge::atomic_bridge_counterparty {
    use std::signer;
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;
    use aptos_framework::aptos_hash::keccak256;
    use aptos_std::smart_table::{Self, SmartTable};
    use moveth::moveth;
    

    const LOCKED: u8 = 1;
    const COMPLETED: u8 = 2;
    const CANCELLED: u8 = 3;

    /// A mapping of bridge transfer IDs to their bridge_transfer
    struct BridgeTransferStore has key, store {
        transfers: SmartTable<vector<u8>, BridgeTransfer>,
        bridge_transfer_locked_events: EventHandle<BridgeTransferLockedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_cancelled_events: EventHandle<BridgeTransferCancelledEvent>,
    }

    struct BridgeTransfer has key, store {
        initiator: vector<u8>, // eth address,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
        state: u8,
    }

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
        signer_cap: account::SignerCapability,
    }

    #[event]
    /// An event triggered upon locking assets for a bridge transfer 
    struct BridgeTransferLockedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: vector<u8>,
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
    
    fun init_module(resource: &signer) {

        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource, @origin_addr);

        move_to(resource, BridgeTransferStore {
            transfers: aptos_std::smart_table::new<vector<u8>, BridgeTransfer>(),
            bridge_transfer_locked_events: account::new_event_handle<BridgeTransferLockedEvent>(resource),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(resource),
            bridge_transfer_cancelled_events: account::new_event_handle<BridgeTransferCancelledEvent>(resource),
        });
        move_to(resource,BridgeConfig {
            moveth_minter: signer::address_of(resource),
            bridge_module_deployer: signer::address_of(resource),
            signer_cap: resource_signer_cap
        });
    }

    #[view]
    public fun bridge_transfers(bridge_transfer_id: vector<u8>): (vector<u8>, address, u64, vector<u8>, u64, u8) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global<BridgeTransferStore>(config_address);
 
        if (!aptos_std::smart_table::contains(&store.transfers, bridge_transfer_id)) {
            abort 0x1; 
        };

        let bridge_transfer_ref = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        (
            bridge_transfer_ref.initiator,
            bridge_transfer_ref.recipient,
            bridge_transfer_ref.amount,
            bridge_transfer_ref.hash_lock,
            bridge_transfer_ref.time_lock,
            bridge_transfer_ref.state
        )
    }
  
    public entry fun lock_bridge_transfer(
        caller: &signer,
        initiator: vector<u8>,
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        time_lock: u64,
        recipient: address,
        amount: u64
    ) acquires BridgeTransferStore {
        //assert!(signer::address_of(caller) == @origin_addr, 1);
        let store = borrow_global_mut<BridgeTransferStore>(@resource_addr);
        let bridge_transfer = BridgeTransfer {
            initiator,
            recipient,
            amount,
            hash_lock,
            time_lock: timestamp::now_seconds() + time_lock,
            state: LOCKED,
        };

        smart_table::add(&mut store.transfers, bridge_transfer_id, bridge_transfer);

        event::emit_event(&mut store.bridge_transfer_locked_events, BridgeTransferLockedEvent {
                amount,
                bridge_transfer_id,
                initiator,
                recipient,
                hash_lock,
                time_lock,
            },
        );
    }
    
    public entry fun complete_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@resource_addr).bridge_module_deployer;
        let resource_signer = account::create_signer_with_capability(&borrow_global<BridgeConfig>(@resource_addr).signer_cap);
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        let computed_hash = keccak256(pre_image);
        assert!(computed_hash == bridge_transfer.hash_lock, 2);
        assert!(bridge_transfer.state == LOCKED, 3);
        bridge_transfer.state = COMPLETED;

        moveth::mint(&resource_signer, bridge_transfer.recipient, bridge_transfer.amount);

        event::emit_event(&mut store.bridge_transfer_completed_events, BridgeTransferCompletedEvent {
                bridge_transfer_id: copy bridge_transfer_id,
                pre_image,
            },
        );
    }

    public entry fun noop_for_testing(_signer: &signer) {
         
    }

    public entry fun pass_data_for_testing(
        _signer: &signer,
        initiator: vector<u8>,
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        time_lock: u64,
        recipient: address,
        amount: u64,

    ) {
         
    }
    
    public fun abort_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>
    ) acquires BridgeTransferStore, BridgeConfig {
        // check that the signer is the bridge_module_deployer
        assert!(signer::address_of(caller) == borrow_global<BridgeConfig>(signer::address_of(caller)).bridge_module_deployer, 1);
        let store = borrow_global_mut<BridgeTransferStore>(signer::address_of(caller));
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        // Ensure the timelock has expired
        assert!(timestamp::now_seconds() > bridge_transfer.time_lock, 2);
        assert!(bridge_transfer.state == LOCKED, 3);

        bridge_transfer.state = CANCELLED;

        event::emit_event(&mut store.bridge_transfer_cancelled_events, BridgeTransferCancelledEvent {
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

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    fun test_complete_bridge_transfer(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
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
        lock_bridge_transfer(
            origin_account,
            initiator,
            bridge_transfer_id,
            hash_lock,
            time_lock,
            recipient,
            amount
        );
        // Verify that the transfer is stored in pending_transfers
        let store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let bridge_transfer: &BridgeTransfer = smart_table::borrow(&store.transfers, bridge_transfer_id);
        assert!(bridge_transfer.recipient == recipient, 2);
        assert!(bridge_transfer.initiator == initiator, 3);
        assert!(bridge_transfer.amount == amount, 5);
        assert!(bridge_transfer.hash_lock == hash_lock, 5);
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
        let store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let bridge_transfer: &BridgeTransfer = smart_table::borrow(&store.transfers, bridge_transfer_id);
        assert!(bridge_transfer.recipient == recipient, 1);
        assert!(bridge_transfer.amount == amount, 2);
        assert!(bridge_transfer.hash_lock == hash_lock, 3);
        assert!(bridge_transfer.initiator == initiator, 4);
    }

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    fun test_get_bridge_transfer_details_from_id(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
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
        lock_bridge_transfer(
            origin_account,
            initiator,
            bridge_transfer_id,
            hash_lock,
            time_lock,
            recipient,
            amount
        );
        let (transfer_initiator, transfer_recipient, transfer_amount, transfer_hash_lock, transfer_time_lock, transfer_state) = bridge_transfers(bridge_transfer_id);
        assert!(transfer_recipient == recipient, 2);
        assert!(transfer_initiator == initiator, 3);
    }
}
