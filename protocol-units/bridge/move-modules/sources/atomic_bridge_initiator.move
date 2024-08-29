module atomic_bridge::atomic_bridge_initiator {
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account::{Self, Account};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::genesis;
    use aptos_framework::timestamp;
    use aptos_std::aptos_hash;
    use aptos_std::smart_table::{Self, SmartTable};
    use std::signer;
    use std::vector;
    use std::bcs;
    use std::debug;
    use moveth::moveth;

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

    struct BridgeTransfer has key, store, drop, copy {
        originator: address,
        recipient: vector<u8>, // eth address
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
        state: u8,
    }

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
    }

    struct BridgeTransferStore has key, store {
        transfers: SmartTable<vector<u8>, BridgeTransfer>,
        nonce: u64,
        bridge_transfer_initiated_events: EventHandle<BridgeTransferInitiatedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_refunded_events: EventHandle<BridgeTransferRefundedEvent>,
    }

    struct BridgeTransferInitiatedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        originator: address,
        recipient: vector<u8>,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    }

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
        });
    }

    #[view]
    public fun bridge_transfers(bridge_transfer_id: vector<u8>): (address, vector<u8>, u64, vector<u8>, u64, u8) acquires BridgeTransferStore, BridgeConfig {
         let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global<BridgeTransferStore>(config_address);
 
        if (!aptos_std::smart_table::contains(&store.transfers, bridge_transfer_id)) {
            abort 0x1; 
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

    public fun initiate_bridge_transfer(
        initiator: &signer,
        recipient: vector<u8>, // eth address
        hash_lock: vector<u8>,
        time_lock: u64,
        amount: u64
    ) : vector<u8> acquires BridgeTransferStore, BridgeConfig {
        let addr = signer::address_of(initiator);
        let asset = moveth::metadata();
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);

        assert!(amount > 0, EINSUFFICIENT_AMOUNT);

        let initiator_store = primary_fungible_store::ensure_primary_store_exists(addr, asset);

        // Check balance of initiator account
        assert!(primary_fungible_store::balance(addr, asset) >= amount, EINSUFFICIENT_BALANCE);
        let bridge_store = primary_fungible_store::ensure_primary_store_exists(@atomic_bridge, asset);
        dispatchable_fungible_asset::transfer(initiator, initiator_store, bridge_store, amount);
        store.nonce = store.nonce + 1;

        // Create a single byte vector by concatenating all components
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&addr));
        vector::append(&mut combined_bytes, recipient);
        vector::append(&mut combined_bytes, hash_lock);
        vector::append(&mut combined_bytes, bcs::to_bytes(&store.nonce));

        let bridge_transfer_id = aptos_std::aptos_hash::keccak256(combined_bytes);

        let bridge_transfer = BridgeTransfer {
            amount: amount,
            originator: addr,
            recipient: recipient,
            hash_lock: hash_lock,
            time_lock: timestamp::now_seconds() + time_lock,
            state: INITIALIZED,
        };

        aptos_std::smart_table::add(&mut store.transfers, bridge_transfer_id, bridge_transfer);

        event::emit_event(&mut store.bridge_transfer_initiated_events, BridgeTransferInitiatedEvent {
            bridge_transfer_id: bridge_transfer_id,
            originator: addr,
            recipient: recipient,
            amount: amount,
            hash_lock: hash_lock,
            time_lock: timestamp::now_seconds() + time_lock,
        });

        bridge_transfer_id
    }

    public fun complete_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
        atomic_bridge: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        assert!(bridge_transfer.state == INITIALIZED, ENOT_INITIALIZED);
        assert!(aptos_std::aptos_hash::keccak256(bcs::to_bytes(&pre_image)) == bridge_transfer.hash_lock, EWRONG_PREIMAGE);
        assert!(timestamp::now_seconds() <= bridge_transfer.time_lock, ETIMELOCK_EXPIRED);

        moveth::burn(atomic_bridge, @atomic_bridge, bridge_transfer.amount);

        // Update the state directly on the mutable reference
        bridge_transfer.state = COMPLETED;

        event::emit_event(&mut store.bridge_transfer_completed_events, BridgeTransferCompletedEvent {
            bridge_transfer_id: copy bridge_transfer_id,
            pre_image: pre_image,
        });
    }

    public fun refund_bridge_transfer(
        account: &signer,
        bridge_transfer_id: vector<u8>,
        atomic_bridge: &signer
    ) acquires BridgeTransferStore, BridgeConfig {
        assert!(signer::address_of(account) == @origin_addr, EINCORRECT_SIGNER);
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let store = borrow_global_mut<BridgeTransferStore>(config_address);
        let bridge_transfer = aptos_std::smart_table::borrow_mut(&mut store.transfers, bridge_transfer_id);

        assert!(bridge_transfer.state == INITIALIZED, ENOT_INITIALIZED);
        assert!(timestamp::now_seconds() > bridge_transfer.time_lock, ENOT_EXPIRED);

        let initiator_addr = bridge_transfer.originator;
        let bridge_addr = signer::address_of(atomic_bridge);
        let asset = moveth::metadata();

        // Transfer amount of asset from atomic bridge primary fungible store to initiator's primary fungible store
        let initiator_store = primary_fungible_store::ensure_primary_store_exists(initiator_addr, asset);
        let bridge_store = primary_fungible_store::ensure_primary_store_exists(@atomic_bridge, asset);
        dispatchable_fungible_asset::transfer(atomic_bridge, bridge_store, initiator_store, bridge_transfer.amount);

        bridge_transfer.state = REFUNDED;

        event::emit_event(&mut store.bridge_transfer_refunded_events, BridgeTransferRefundedEvent {
            bridge_transfer_id: copy bridge_transfer_id,
        });

        aptos_std::smart_table::remove(&mut store.transfers, bridge_transfer_id);
    }

    #[test_only]
    public fun init_test(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) {
        genesis::setup();
        moveth::init_for_test(creator);
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

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    public fun test_initiate_bridge_transfer(
        sender: &signer,
        creator: &signer,
        aptos_framework: &signer,
        atomic_bridge: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        init_test(sender, creator, aptos_framework, atomic_bridge);

        let recipient = b"recipient_address";
        let hash_lock = b"hash_lock_value";
        let time_lock = 1000;
        let amount = 1000;

        // Mint amount of tokens to sender
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        let addr = signer::address_of(sender);
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(transfer.amount == amount, 200);
        assert!(transfer.originator == addr, 201);
        assert!(transfer.recipient == b"recipient_address", 202);
        assert!(transfer.hash_lock == b"hash_lock_value", 203);
        assert!(transfer.time_lock == timestamp::now_seconds() + time_lock, 204);
        assert!(transfer.state == INITIALIZED, 205);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    #[expected_failure ( abort_code = EINSUFFICIENT_BALANCE )]
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

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        let addr = signer::address_of(sender);
        let store = borrow_global<BridgeTransferStore>(addr);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(transfer.amount == amount, 200);
        assert!(transfer.originator == addr, 201);
        assert!(transfer.recipient == b"recipient_address", 202);
        assert!(transfer.hash_lock == b"hash_lock_value", 203);
        assert!(transfer.time_lock == timestamp::now_seconds() + time_lock, 204);
        assert!(transfer.state == INITIALIZED, 205);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
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
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            pre_image,
            atomic_bridge
        );
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        // complete bridge doesn't delete the transfer from the store
        assert!(aptos_std::smart_table::contains(&store.transfers, copy bridge_transfer_id), 300);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
    #[expected_failure ( abort_code = EWRONG_PREIMAGE )]
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
        let time_lock = 1000;
        let amount = 1000;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            wrong_pre_image,
            atomic_bridge
        );

        let addr = signer::address_of(sender);
        let store = borrow_global<BridgeTransferStore>(addr);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(transfer.state == COMPLETED, 300);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @origin_addr, atomic_bridge = @atomic_bridge)]
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
        let time_lock = 1;
        let amount = 1000;

        // Mint amount of tokens to sender
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        aptos_framework::timestamp::fast_forward_seconds(time_lock + 2);

        refund_bridge_transfer(
            sender,
            bridge_transfer_id,
            atomic_bridge
        );

        let addr = signer::address_of(sender);
        let asset = moveth::metadata();
        assert!(primary_fungible_store::balance(addr, asset) == amount, 0);
        let bridge_addr = signer::address_of(atomic_bridge);
        let store = borrow_global<BridgeTransferStore>(bridge_addr);
        assert!(!aptos_std::smart_table::contains(&store.transfers, copy bridge_transfer_id), 300);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @origin_addr, atomic_bridge = @atomic_bridge)]
    #[expected_failure ( abort_code = ENOT_INITIALIZED )]
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
        let time_lock = 1000;
        let amount = 1000;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
            amount
        );

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            pre_image,
            atomic_bridge
        );

        refund_bridge_transfer(
            sender,
            bridge_transfer_id,
            atomic_bridge
        );

        let addr = signer::address_of(sender);
        let store = borrow_global<BridgeTransferStore>(addr);
        let transfer = aptos_std::smart_table::borrow(&store.transfers, bridge_transfer_id);

        assert!(transfer.state == COMPLETED, 300);
    }

    #[test(creator = @moveth, aptos_framework = @0x1, sender = @0xdaff, atomic_bridge = @atomic_bridge)]
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
        let time_lock = 1000;
        let amount = 1000;
        let sender_address = signer::address_of(sender);
        moveth::mint(atomic_bridge, sender_address, amount);

        let bridge_transfer_id = initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            time_lock,
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
            atomic_bridge
        );
        aptos_std::debug::print(&transfer_state);
    }
}