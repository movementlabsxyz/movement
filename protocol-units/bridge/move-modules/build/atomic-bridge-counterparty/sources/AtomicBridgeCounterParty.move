module MoveBridge::AtomicBridgeCounterParty {
    use std::signer;
    use std::event;
    use std::vector;
    use std::timestamp;
    use aptos_std::smart_table::{Self, SmartTable};

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

    public fun initialize(owner: &signer) {
        let bridge_transfer_store = BridgeTransferStore {
            pending_transfers: smart_table::new(),
            completed_transfers: smart_table::new(),
            aborted_transfers: smart_table::new(),
        };
        move_to(owner, bridge_transfer_store);
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
    ) acquires BridgeTransferStore {
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(initiator));
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);
        assert!(details.recipient != @0x0, 1); // Ensure the transfer exists
        smart_table::add(&mut bridge_store.aborted_transfers, bridge_transfer_id, details);
        event::emit(
            BridgeTransferCancelledEvent {
                bridge_transfer_id,
            },
        );
    }
}