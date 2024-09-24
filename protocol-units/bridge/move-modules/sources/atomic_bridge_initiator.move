module atomic_bridge::atomic_bridge_initiator {
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account::{Self, Account};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::genesis;
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;
    use aptos_std::aptos_hash;
    use aptos_std::smart_table::{Self, SmartTable};
    use std::signer;
    use std::vector;
    use std::bcs;
    use std::debug;
    use moveth::moveth;
    use atomic_bridge::atomic_bridge_counterparty;
    

    const INITIALIZED: u8 = 1;
    const COMPLETED: u8 = 2;
    const REFUNDED: u8 = 3;

    const EINSUFFICIENT_AMOUNT: u64 = 0;
    const EINSUFFICIENT_BALANCE: u64 = 1;
    const EDOES_NOT_EXIST: u64 = 2;
    const EWRONG_PREIMAGE: u64 = 3;
    const ENOT_INITIALIZED: u64 = 4;
    const ETIMELOCK_EXPIRED: u64 = 5;
    const ENOT_EXPIRED: u64 = 6;
    const EINCORRECT_SIGNER: u64 = 7;
    const EWRONG_RECIPIENT: u64 = 8;
    const EWRONG_ORIGINATOR: u64 = 9;
    const EWRONG_AMOUNT: u64 = 10;
    const EWRONG_HASHLOCK: u64 = 11;

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
        time_lock_duration: u64,
    }

    /// A mapping of bridge transfer IDs to their bridge_transfer
    struct BridgeTransferStore has key, store {
        transfers: SmartTable<vector<u8>, BridgeTransfer>,
        nonce: u64,
        bridge_transfer_initiated_events: EventHandle<BridgeTransferInitiatedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_refunded_events: EventHandle<BridgeTransferRefundedEvent>,
    }

    struct BridgeTransfer has key, store, drop {
        originator: address,
        recipient: vector<u8>, // eth address
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
        state: u8,
    }

    #[event]
    struct BridgeTransferInitiatedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        originator: address,
        recipient: vector<u8>,
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
    struct BridgeTransferRefundedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }

    fun init_module(deployer: &signer) {
        let deployer_addr = signer::address_of(deployer);

        move_to(deployer, BridgeTransferStore {
            transfers: aptos_std::smart_table::new<vector<u8>, BridgeTransfer>(),
            nonce: 0,
            bridge_transfer_initiated_events: account::new_event_handle<BridgeTransferInitiatedEvent>(deployer),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(deployer),
            bridge_transfer_refunded_events: account::new_event_handle<BridgeTransferRefundedEvent>(deployer),
        });

        move_to(deployer, BridgeConfig {
            moveth_minter: signer::address_of(deployer),
            bridge_module_deployer: signer::address_of(deployer),
            time_lock_duration: 48 * 60 * 60, // 48 hours
        });
    }

    #[view]
    public fun bridge_transfers(bridge_transfer_id: vector<u8>): (address, vector<u8>, u64, vector<u8>, u64, u8) acquires BridgeTransferStore, BridgeConfig {
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

    public entry fun initiate_bridge_transfer(
        originator: &signer,
        recipient: vector<u8>, // eth address
        hash_lock: vector<u8>,
        amount: u64
    ) acquires BridgeTransferStore, BridgeConfig {
        let originator_addr = signer::address_of(originator);
        let asset = moveth::metadata();
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);

        assert!(amount > 0, EINSUFFICIENT_AMOUNT);

        let originator_store = primary_fungible_store::ensure_primary_store_exists(originator_addr, asset);

        // Check balance of originator account
        assert!(primary_fungible_store::balance(originator_addr, asset) >= amount, EINSUFFICIENT_BALANCE);
        let bridge_store = primary_fungible_store::ensure_primary_store_exists(@atomic_bridge, asset);
        
        store.nonce = store.nonce + 1;

        // Create a single byte vector by concatenating all components
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&originator_addr));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&store.nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        let time_lock = borrow_global<BridgeConfig>(@atomic_bridge).time_lock_duration;

        let bridge_transfer = BridgeTransfer {
            amount: amount,
            originator: originator_addr,
            recipient: recipient,
            hash_lock: hash_lock,
            time_lock: timestamp::now_seconds() + time_lock, 
            state: INITIALIZED,
        };

        aptos_std::smart_table::add(&mut store.transfers, bridge_transfer_id, bridge_transfer);
        atomic_bridge_counterparty::burn_moveth(originator_addr, amount);

        event::emit_event(&mut store.bridge_transfer_initiated_events, BridgeTransferInitiatedEvent {
            bridge_transfer_id: bridge_transfer_id,
            originator: originator_addr,
            recipient: recipient,
            amount: amount,
            hash_lock: hash_lock,
            time_lock: time_lock,
        });
    }

    public entry fun complete_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);
 
        assert!(bridge_transfer.state == INITIALIZED, ENOT_INITIALIZED);
        assert!(aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image)) == bridge_transfer.hash_lock, EWRONG_PREIMAGE);
        assert!(timestamp::now_seconds() <= bridge_transfer.time_lock, ETIMELOCK_EXPIRED);
        
        bridge_transfer.state = COMPLETED;

        event::emit_event(&mut store.bridge_transfer_completed_events, BridgeTransferCompletedEvent {
            bridge_transfer_id: copy bridge_transfer_id,
            pre_image: pre_image,
        });
    }

    public entry fun refund_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>,
    ) acquires BridgeTransferStore, BridgeConfig {
        assert!(signer::address_of(account) == @origin_addr, EINCORRECT_SIGNER);
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        assert!(bridge_transfer.state == INITIALIZED, ENOT_INITIALIZED);
        assert!(timestamp::now_seconds() > bridge_transfer.time_lock, ENOT_EXPIRED);

        let originator_addr = bridge_transfer.originator;
        let asset = moveth::metadata();

        // Transfer amount of asset from atomic bridge primary fungible store to originator's primary fungible store
        let initiator_store = primary_fungible_store::ensure_primary_store_exists(originator_addr, asset);
        let bridge_store = primary_fungible_store::ensure_primary_store_exists(@atomic_bridge, asset);

        atomic_bridge_counterparty::mint_moveth(bridge_transfer.originator, bridge_transfer.amount);

        bridge_transfer.state = REFUNDED;

        event::emit_event(&mut store.bridge_transfer_refunded_events, BridgeTransferRefundedEvent {
            bridge_transfer_id: copy bridge_transfer_id,
        });

        //aptos_std::smart_table::remove(&mut store.transfers, bridge_transfer_id);
    }

    public fun get_time_lock_duration(): u64 acquires BridgeConfig {
        let config = borrow_global<BridgeConfig>(@atomic_bridge);
        config.time_lock_duration
    }

    public entry fun set_time_lock_duration(resource: &signer, time_lock_duration: u64) acquires BridgeConfig {
        let config = borrow_global_mut<BridgeConfig>(signer::address_of(resource));
        // Check if the signer is the deployer (the original initializer)
        assert!(signer::address_of(resource) == config.bridge_module_deployer, EINCORRECT_SIGNER);

        config.time_lock_duration = time_lock_duration;
    }

    #[test_only]
    public fun init_test(
        sender: &signer,
        origin_account: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) {
        genesis::setup();
        moveth::init_for_test(atomic_bridge);
        atomic_bridge_counterparty::set_up_test(origin_account, atomic_bridge);
        let bridge_addr = signer::address_of(atomic_bridge);
        account::create_account_if_does_not_exist(bridge_addr);
        init_module(atomic_bridge);
        assert!(exists<BridgeTransferStore>(bridge_addr), EDOES_NOT_EXIST);
    }

    #[test(sender = @0xdaff)]
    public fun test_initialize (
        sender: &signer
    ) acquires BridgeTransferStore {
        let addr = signer::address_of(sender);

        // Ensure Account resource exists for the sender
        account::create_account_if_does_not_exist(addr);

        init_module(sender);

        assert!(exists<BridgeTransferStore>(addr), 999);

        let addr = signer::address_of(sender);
        let store = borrow_global<BridgeTransferStore>(addr);

        assert!(aptos_std::smart_table::length(&store.transfers) == 0, 100);
        assert!(store.nonce == 0, 101);
    }
    
    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_initiate_bridge_transfer(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let sender_addr = signer::address_of(sender);
        let recipient = b"recipient_address";
        let hash_lock = b"hash_lock_value";
        let time_lock:u64 = 1000;
        let amount:u64 = 1000;
        let nonce:u64 = 1;

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_addr));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        // Mint amount of tokens to sender
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        let addr = signer::address_of(sender);
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        // The timelock is internally doubled by the initiator module
        let expected_time_lock = timestamp::now_seconds() + get_time_lock_duration();

        assert!(transfer.amount == amount, 200);
        assert!(transfer.originator == addr, 201);
        assert!(transfer.recipient == b"recipient_address", 202);
        assert!(transfer.hash_lock == b"hash_lock_value", 203);
        assert!(transfer.time_lock == expected_time_lock, 204);
        assert!(transfer.state == INITIALIZED, 205);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_get_time_lock_duration(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) acquires BridgeConfig {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        moveth::init_for_test(creator);
        let bridge_addr = signer::address_of(atomic_bridge);
        account::create_account_if_does_not_exist(bridge_addr);

        init_module(atomic_bridge);

        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 48 * 60 * 60, 0);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_set_time_lock_duration(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) acquires BridgeConfig {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        moveth::init_for_test(creator);
        let bridge_addr = signer::address_of(atomic_bridge);
        account::create_account_if_does_not_exist(bridge_addr);

        init_module(atomic_bridge);

        let new_time_lock_duration = 42;
        set_time_lock_duration(atomic_bridge, new_time_lock_duration);

        let time_lock_duration = get_time_lock_duration();
        assert!(time_lock_duration == 42, 0);
    }
    
    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    #[expected_failure (abort_code = EINSUFFICIENT_BALANCE, location = Self)]
    public fun test_initiate_bridge_transfer_no_moveth(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        moveth::init_for_test(creator);
        let bridge_addr = signer::address_of(atomic_bridge);
        account::create_account_if_does_not_exist(bridge_addr);
        init_module(atomic_bridge);
        assert!(exists<BridgeTransferStore>(bridge_addr), EDOES_NOT_EXIST);

        let recipient = b"recipient_address";
        let hash_lock = b"hash_lock_value";
        let time_lock = 1000;
        let amount = 1000;

        // Do not mint tokens to sender; sender has no MovETH

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );
    }

    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_complete_bridge_transfer(
        sender: &signer,
        atomic_bridge: &signer,
        creator: &signer,
        aptos_framework: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let recipient = b"recipient_address";
        let pre_image = b"pre_image_value";
        let hash_lock = aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image));
        assert!(aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image)) == hash_lock, EWRONG_PREIMAGE);
        let time_lock = 1000;
        let amount = 1000;
        let nonce = 1;
        let sender_addr = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_addr, amount);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_addr));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            pre_image,
        );
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        // complete bridge doesn't delete the transfer from the store
        assert!(aptos_std::smart_table::contains(&store.transfers, copy bridge_transfer_id), 300);
        let bridge_transfer: &BridgeTransfer = smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(bridge_transfer.recipient == recipient, EWRONG_RECIPIENT);
        assert!(bridge_transfer.originator == sender_addr, EWRONG_ORIGINATOR);
        assert!(bridge_transfer.amount == amount, EWRONG_AMOUNT);
        assert!(bridge_transfer.hash_lock == hash_lock, EWRONG_HASHLOCK);
    }

    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    #[expected_failure(abort_code = EWRONG_PREIMAGE, location = Self)]
    public fun test_complete_bridge_transfer_wrong_preimage(
        sender: &signer,
        atomic_bridge: &signer,
        creator: &signer,
        aptos_framework: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let recipient = b"recipient_address";
        let pre_image = b"pre_image_value";
        let wrong_pre_image = b"wrong_pre_image_value";
        let hash_lock = aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image));
        let amount = 1000;
        let nonce = 1;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_address));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            wrong_pre_image,
        );
    }

    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @origin_addr, atomic_bridge = @atomic_bridge)]
    // see tracking issue https://github.com/movementlabsxyz/movement/issues/272
    public fun test_refund_bridge_transfer(
        sender: &signer,
        atomic_bridge: &signer,
        creator: &signer,
        aptos_framework: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);
        let recipient = b"recipient_address";
        let hash_lock = b"hash_lock_value";
        let amount = 1000;
        let nonce = 1;

        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_address));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        // Push timestamp forward by double the timelock (since initiator doubles it)
        let time_lock = get_time_lock_duration();
        aptos_framework::timestamp::fast_forward_seconds(time_lock + 2);

        refund_bridge_transfer(
            sender,
            bridge_transfer_id,
        );

        let addr = signer::address_of(sender);
        let asset = moveth::metadata();
        assert!(primary_fungible_store::balance(addr, asset) == amount, 0);
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        assert!(aptos_std::smart_table::contains(&store.transfers, copy bridge_transfer_id), 300);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(transfer.state == REFUNDED, 300);
    }

    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @origin_addr, atomic_bridge = @atomic_bridge)]
    #[expected_failure(abort_code = ENOT_INITIALIZED, location = Self)]
    public fun test_refund_completed_transfer(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let recipient = b"recipient_address";
        let pre_image = b"pre_image_value";
        let hash_lock = aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image));
        assert!(aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image)) == hash_lock, 5);
        let amount = 1000;
        let nonce = 1;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_address));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            pre_image,
        );

        refund_bridge_transfer(
            sender,
            bridge_transfer_id,
        );
    }

    #[test(creator = @origin_addr, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_bridge_transfers_view(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let recipient = b"recipient_address";
        let pre_image = b"pre_image_value";
        let hash_lock = aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image));
        assert!(aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image)) == hash_lock, 5);
        let amount = 1000;
        let nonce = 1;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&sender_address));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        aptos_std::debug::print(&bridge_transfer_id);
        // returns a valid transfer
        let (transfer_originator, transfer_recipient, transfer_amount, transfer_hash_lock, transfer_time_lock, transfer_state) = bridge_transfers(bridge_transfer_id);

        assert!(transfer_state == INITIALIZED, 6);
        aptos_std::debug::print(&transfer_state);
        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            pre_image,
        );
        aptos_std::debug::print(&transfer_state);
    }
}
