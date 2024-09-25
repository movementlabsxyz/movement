module atomic_bridge::atomic_bridge_counterparty {
    friend atomic_bridge::atomic_bridge_initiator;

    use std::signer;
    use std::vector;
    use aptos_framework::account::{Self, SignerCapability};
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

    const EINCORRECT_SIGNER: u64 = 1;
    const EWRONG_PREIMAGE: u64 = 2;
    const ETRANSFER_NOT_LOCKED: u64 = 3;
    const ETIMELOCK_NOT_EXPIRED: u64 = 4;
    const EWRONG_RECIPIENT: u64 = 5;
    const EWRONG_ORIGINATOR: u64 = 6;
    const EWRONG_AMOUNT: u64 = 7;
    const EWRONG_HASHLOCK: u64 = 8;
    const ENO_RESULT: u64 = 9;
    const EWRONG_STATE: u64 = 10;
    const ETIMELOCK_EXPIRED: u64 = 11;

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
        signer_cap: account::SignerCapability,
        time_lock_duration: u64,
    }

    /// A mapping of bridge transfer IDs to their bridge_transfer
    struct BridgeTransferStore has key, store {
        transfers: SmartTable<vector<u8>, BridgeTransfer>,
        // Bridge Transfer Store does not use nonces
        bridge_transfer_locked_events: EventHandle<BridgeTransferLockedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_cancelled_events: EventHandle<BridgeTransferCancelledEvent>,
    }

    struct BridgeTransfer has key, store {
        originator: vector<u8>, // eth address,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
        state: u8,
    }

    #[event]
    struct BridgeTransferLockedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        originator: vector<u8>,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    #[event]
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    }

    #[event]
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
        move_to(resource, BridgeConfig {
            moveth_minter: signer::address_of(resource),
            bridge_module_deployer: signer::address_of(resource),
            signer_cap: resource_signer_cap,
            time_lock_duration: 24 * 60 * 60  // Default 24 hours
        });
    }

    public fun get_time_lock_duration(): u64 acquires BridgeConfig {
        let config = borrow_global<BridgeConfig>(@atomic_bridge);
        config.time_lock_duration
    }

    public entry fun set_time_lock_duration(origin: &signer, time_lock_duration: u64) acquires BridgeConfig {
        let config = borrow_global_mut<BridgeConfig>(@resource_addr);
        // Check if the signer is the deployer (the original initializer)
        assert!(signer::address_of(origin) == @origin_addr, EINCORRECT_SIGNER);

        config.time_lock_duration = time_lock_duration;
    }

    public(friend) fun mint_moveth(to: address, amount: u64) acquires BridgeConfig {
        let config = borrow_global<BridgeConfig>(@atomic_bridge);
        moveth::mint(&account::create_signer_with_capability(&config.signer_cap), to, amount);
    }

    public(friend) fun burn_moveth(from: address, amount: u64) acquires BridgeConfig {
        let config = borrow_global<BridgeConfig>(@atomic_bridge);
        moveth::burn(&account::create_signer_with_capability(&config.signer_cap), from, amount);
    }

    #[view]
    public fun bridge_transfers(bridge_transfer_id: vector<u8>): (vector<u8>, address, u64, vector<u8>, u64, u8) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global<BridgeTransferStore>(config_address);
 
        if (!aptos_std::smart_table::contains(&store.transfers, bridge_transfer_id)) {
            abort 0x1
        };

        let bridge_transfer_ref = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        (
            bridge_transfer_ref.originator,
            bridge_transfer_ref.recipient,
            bridge_transfer_ref.amount,
            bridge_transfer_ref.hash_lock,
            bridge_transfer_ref.time_lock,
            bridge_transfer_ref.state
        )
    }

        public entry fun lock_bridge_transfer(
            account: &signer,
            originator: vector<u8>, //eth address
            bridge_transfer_id: vector<u8>,
            hash_lock: vector<u8>,
            recipient: address,
            amount: u64
        ) acquires BridgeTransferStore, BridgeConfig {
            // Use the configured time lock duration from BridgeConfig
            let config = borrow_global<BridgeConfig>(@atomic_bridge);
            let time_lock = timestamp::now_seconds() + config.time_lock_duration;

            assert!(signer::address_of(account) == @origin_addr, EINCORRECT_SIGNER);
            let store = borrow_global_mut<BridgeTransferStore>(@resource_addr);
            let bridge_transfer = BridgeTransfer {
                originator,
                recipient,
                amount,
                hash_lock,
                time_lock,
                state: LOCKED,
            };
            smart_table::add(&mut store.transfers, bridge_transfer_id, bridge_transfer);

            event::emit_event(&mut store.bridge_transfer_locked_events, BridgeTransferLockedEvent {
                    amount,
                    bridge_transfer_id,
                    originator,
                    recipient,
                    hash_lock,
                    time_lock,
                },
            );
        }

    public entry fun complete_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@resource_addr).bridge_module_deployer;
        let resource_signer = account::create_signer_with_capability(&borrow_global<BridgeConfig>(@resource_addr).signer_cap);
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        let computed_hash = keccak256(pre_image);
        assert!(computed_hash == bridge_transfer.hash_lock, EWRONG_PREIMAGE);
        assert!(bridge_transfer.state == LOCKED, ETRANSFER_NOT_LOCKED);
        assert!(timestamp::now_seconds() <= bridge_transfer.time_lock, ETIMELOCK_EXPIRED);
        bridge_transfer.state = COMPLETED;

        moveth::mint(&resource_signer, bridge_transfer.recipient, bridge_transfer.amount);

        event::emit_event(&mut store.bridge_transfer_completed_events, BridgeTransferCompletedEvent {
                bridge_transfer_id: copy bridge_transfer_id,
                pre_image,
            },
        );
    }

    public entry fun abort_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>
    ) acquires BridgeTransferStore, BridgeConfig {
        // check that the signer is the bridge_module_deployer
        assert!(signer::address_of(account) == @origin_addr, EINCORRECT_SIGNER);
        let config_address = borrow_global<BridgeConfig>(@resource_addr).bridge_module_deployer;
        let resource_signer = account::create_signer_with_capability(&borrow_global<BridgeConfig>(@resource_addr).signer_cap);
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        // Ensure the timelock has expired
        assert!(timestamp::now_seconds() > bridge_transfer.time_lock, ETIMELOCK_NOT_EXPIRED);
        assert!(bridge_transfer.state == LOCKED, ETRANSFER_NOT_LOCKED);

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
        let originator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();
        
        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); 
        let time_lock = 3600;
        let amount = 100;
        lock_bridge_transfer(
            origin_account,
            originator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount
        );
        // Verify that the transfer is stored in pending_transfers
        let store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let bridge_transfer: &BridgeTransfer = smart_table::borrow(&store.transfers, bridge_transfer_id);
        let time_lock_duration = borrow_global<BridgeConfig>(@atomic_bridge).time_lock_duration;

        let expected_time_lock =  time_lock_duration;

        assert!(bridge_transfer.recipient == recipient, EWRONG_RECIPIENT);
        assert!(bridge_transfer.originator == originator, EWRONG_ORIGINATOR);
        assert!(bridge_transfer.amount == amount, EWRONG_AMOUNT);
        assert!(bridge_transfer.hash_lock == hash_lock, EWRONG_HASHLOCK);
        assert!(bridge_transfer.time_lock == timestamp::now_seconds() + expected_time_lock, 420);

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

        assert!(bridge_transfer.recipient == recipient, EWRONG_RECIPIENT);
        assert!(bridge_transfer.amount == amount, EWRONG_AMOUNT);
        assert!(bridge_transfer.hash_lock == hash_lock, EWRONG_HASHLOCK);
        assert!(bridge_transfer.originator == originator, EWRONG_ORIGINATOR);
    }

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    #[expected_failure (abort_code = ETIMELOCK_EXPIRED, location = Self)]
    fun test_complete_bridge_transfer_expired(
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
        let originator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();
        
        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); 
        let amount = 100;
        lock_bridge_transfer(
            origin_account,
            originator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount
        );
        // Verify that the transfer is stored in pending_transfers
        let store = borrow_global<BridgeTransferStore>(signer::address_of(&resource_addr));
        let bridge_transfer: &BridgeTransfer = smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(bridge_transfer.recipient == recipient, EWRONG_RECIPIENT);
        assert!(bridge_transfer.originator == originator, EWRONG_ORIGINATOR);
        assert!(bridge_transfer.amount == amount, EWRONG_AMOUNT);
        assert!(bridge_transfer.hash_lock == hash_lock, EWRONG_HASHLOCK);

        let config = borrow_global<BridgeConfig>(@atomic_bridge);

        aptos_framework::timestamp::fast_forward_seconds(config.time_lock_duration + 2);

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

        assert!(bridge_transfer.recipient == recipient, EWRONG_RECIPIENT);
        assert!(bridge_transfer.amount == amount, EWRONG_AMOUNT);
        assert!(bridge_transfer.hash_lock == hash_lock, EWRONG_HASHLOCK);
        assert!(bridge_transfer.originator == originator, EWRONG_ORIGINATOR);
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
        let originator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();
        
        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); 
        let amount = 100;
        lock_bridge_transfer(
            origin_account,
            originator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount
        );
        let (transfer_originator, transfer_recipient, transfer_amount, transfer_hash_lock, transfer_time_lock, transfer_state) = bridge_transfers(bridge_transfer_id);

        assert!(transfer_recipient == recipient, EWRONG_RECIPIENT);
        assert!(transfer_originator == originator, EWRONG_ORIGINATOR);
    }

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter, malicious=@0xface)]
    #[expected_failure (abort_code = EINCORRECT_SIGNER)]
    fun test_malicious_lock(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
        malicious: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        set_up_test(origin_account, &resource_addr);

        timestamp::set_time_has_started_for_testing(&aptos_framework);
        moveth::init_for_test(moveth);
        let receiver_address = @0xdada;
        let originator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();
        
        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); 
        let time_lock = 3600;
        let amount = 100;
        lock_bridge_transfer(
            malicious,
            originator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount
        );
        let (transfer_originator, transfer_recipient, transfer_amount, transfer_hash_lock, transfer_time_lock, transfer_state) = bridge_transfers(bridge_transfer_id);
        assert!(transfer_recipient == recipient, 2);
        assert!(transfer_originator == originator, 3);
    }

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    public fun test_get_time_lock_duration(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
    ) acquires BridgeConfig {
        set_up_test(origin_account, &resource_addr);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        moveth::init_for_test(moveth);

        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 24 * 60 * 60, 1);
    }

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    public fun test_set_time_lock_duration(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
    ) acquires BridgeConfig {
        set_up_test(origin_account, &resource_addr);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        moveth::init_for_test(moveth);

        // Timelock should be at default before setting
        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 24 * 60 * 60, 1);

        // Set the timelock to 42
        set_time_lock_duration(origin_account, 42);
        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 42, 2);
    } 

    #[test(origin_account = @origin_addr, resource_addr = @resource_addr, aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, admin = @admin, client = @0xdca, master_minter = @master_minter)]
    #[expected_failure (abort_code = EINCORRECT_SIGNER)]
    public fun test_should_fail_set_time_lock_duration_wrong_signer(
        origin_account: &signer,
        resource_addr: signer,
        client: &signer,
        aptos_framework: signer,
        master_minter: &signer, 
        creator: &signer,
        moveth: &signer,
    ) acquires BridgeConfig {
        set_up_test(origin_account, &resource_addr);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        moveth::init_for_test(moveth);

        // Timelock should be at default before setting
        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 24 * 60 * 60, 1);

        // Set the timelock to 42
        set_time_lock_duration(client, 42);
        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 42, 2);
    }
}
