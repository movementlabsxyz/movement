module MoveBridge::AtomicBridgeCounterParty {
    use std::signer;
    use std::event;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_std::smart_table::{Self, SmartTable};
    use MOVETH::moveth;

    /// A mapping of bridge transfer IDs to their details
    struct BridgeTransferStore has key, store {
        pending_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        completed_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        aborted_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
    }

    struct BridgeTransferDetails has key, store {
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    struct BridgeConfig has key {
        moveth_minter: address,
        bridge_module_deployer: address,
    }

    #[event]
    /// An event triggered upon locking assets for a bridge transfer 
    struct BridgeTransferAssetsLockedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: address,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    #[event]
    /// An event triggered upon completing a bridge transfer
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        secret: vector<u8>,
    }

    #[event]
    /// An event triggered upon cancelling a bridge transfer
    struct BridgeTransferCancelledEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }

    entry fun initialize(owner: &signer, moveth_minter: address) {
        let bridge_transfer_store = BridgeTransferStore {
            pending_transfers: smart_table::new(),
            completed_transfers: smart_table::new(),
            aborted_transfers: smart_table::new(),
        };
        let bridge_config = BridgeConfig {
            moveth_minter,
            bridge_module_deployer: signer::address_of(owner),
        };
        move_to(owner, bridge_transfer_store);
        move_to(owner, bridge_config);
    }

    public fun lock_bridge_transfer_assets(
        initiator: &signer,
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        time_lock: u64,
        recipient: address,
        amount: u64
    ): bool acquires BridgeTransferStore {
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(initiator));
        let details = BridgeTransferDetails {
            recipient,
            amount,
            hash_lock,
            time_lock: timestamp::now_seconds() + time_lock,
        };

        smart_table::add(&mut bridge_store.pending_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferAssetsLockedEvent {
                bridge_transfer_id,
                initiator: signer::address_of(initiator),
                recipient,
                amount,
                hash_lock,
                time_lock,
            },
        );

        true
    }

    public fun complete_bridge_transfer(
        initiator: &signer,
        bridge_transfer_id: vector<u8>,
        secret: vector<u8>
    ) acquires BridgeTransferStore {
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(initiator));
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);
        assert!(details.recipient != @0x0, 1); 

        // check secret against details.hash_lock


        // Mint MOVETH tokens to the recipient
        moveth::mint(initiator, details.recipient, details.amount);

        smart_table::add(&mut bridge_store.completed_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferCompletedEvent {
                bridge_transfer_id,
                secret,
            },
        );
    }

    public fun abort_bridge_transfer(
        initiator: &signer,
        bridge_transfer_id: vector<u8>
    ) acquires BridgeTransferStore, BridgeConfig {
        // check that the signer is the bridge_module_deployer
        assert!(signer::address_of(initiator) == borrow_global<BridgeConfig>(signer::address_of(initiator)).bridge_module_deployer, 1);
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(initiator));
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);
        assert!(details.recipient != @0x0, 1); 
        smart_table::add(&mut bridge_store.aborted_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferCancelledEvent {
                bridge_transfer_id,
            },
        );
    }

    #[test(creator = @MoveBridge)]
    fun test_initialize(
        creator: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        let owner = signer::address_of(creator);
        let moveth_minter = @0x1; 
        initialize(creator, moveth_minter);

        // Verify that the BridgeTransferStore and BridgeConfig have been initialized
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let bridge_config = borrow_global<BridgeConfig>(signer::address_of(creator));

        assert!(bridge_config.moveth_minter == moveth_minter, 1);
        assert!(bridge_config.bridge_module_deployer == owner, 2);
    }

    #[test(creator = @MoveBridge)]
    fun test_lock_bridge_transfer_assets(
        creator: &signer,
    ) acquires BridgeTransferStore {
        timestamp::set_time_has_started_for_testing(creator);
        let initiator = signer::address_of(creator); 
        let recipient = @0xface; 
        let moveth_minter = @0xdead; 
        initialize(creator, moveth_minter);

        let bridge_transfer_id = b"transfer1";
        let hash_lock = b"hashlock1";
        let time_lock = 3600;
        let amount = 100;

        let result = lock_bridge_transfer_assets(
            creator,
            bridge_transfer_id,
            hash_lock,
            time_lock,
            recipient,
            amount
        );

        assert!(result, 1);

        //Verify that the transfer is stored in pending_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store.pending_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 2);
        assert!(transfer_details.amount == amount, 3);
        assert!(transfer_details.hash_lock == hash_lock, 4);

       let secret = b"secret"; 
       complete_bridge_transfer(
           creator,
           bridge_transfer_id,
           secret
       );

        // Verify that the transfer is stored in completed_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store.completed_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 1);
        assert!(transfer_details.amount == amount, 2);
    }
}