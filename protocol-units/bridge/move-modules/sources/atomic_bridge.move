module atomic_bridge::atomic_bridge_counterparty {
    use std::signer;
    use std::event;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_framework::aptos_hash::keccak256;
    use aptos_std::smart_table::{Self, SmartTable};
    use moveth::moveth;

    /// A mapping of bridge transfer IDs to their details
    struct BridgeTransferStore has key, store {
        pending_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        completed_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
        aborted_transfers: SmartTable<vector<u8>, BridgeTransferDetails>,
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
    /// An event triggered upon canceling a bridge transfer
    struct BridgeTransferCancelledEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }
    
    entry fun init_module(deployer: &signer) {
        let bridge_transfer_store = BridgeTransferStore {
            pending_transfers: smart_table::new(),
            completed_transfers: smart_table::new(),
            aborted_transfers: smart_table::new(),
        };
        let bridge_config = BridgeConfig {
            moveth_minter: signer::address_of(deployer),
            bridge_module_deployer: signer::address_of(deployer),
        };
        move_to(deployer, bridge_transfer_store);
        move_to(deployer, bridge_config);
    }
    
    public fun lock_bridge_transfer_assets(
        caller: &signer,
        initiator: vector<u8>, //eth address
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        time_lock: u64,
        recipient: address,
        amount: u64
    ): bool acquires BridgeTransferStore {
        let bridge_store = borrow_global_mut<BridgeTransferStore>(signer::address_of(caller));
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

        true
    }
    
    public fun complete_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>
    ) acquires BridgeTransferStore, BridgeConfig {
        let config_address = borrow_global<BridgeConfig>(@atomic_bridge).bridge_module_deployer;
        let bridge_store = borrow_global_mut<BridgeTransferStore>(config_address);
        let details: BridgeTransferDetails = smart_table::remove(&mut bridge_store.pending_transfers, bridge_transfer_id);
        // Check secret against details.hash_lock
        let computed_hash = keccak256(pre_image);
        assert!(computed_hash == details.hash_lock, 2);

        // Mint moveth tokens to the recipient, caller must me a minter of moveth
        moveth::mint(caller, details.recipient, details.amount);

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

    
    #[test(creator = @atomic_bridge)]
    fun test_init_module(
        creator: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        let owner = signer::address_of(creator);
        let moveth_minter = @0x1; 
        init_module(creator);

        // Verify that the BridgeTransferStore and BridgeConfig have been init_moduled
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let bridge_config = borrow_global<BridgeConfig>(signer::address_of(creator));

        assert!(bridge_config.moveth_minter == signer::address_of(creator), 1);
        assert!(bridge_config.bridge_module_deployer == owner, 2);
    }

    use std::debug;
    use std::string::{String, utf8};
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::primary_fungible_store;

    #[test(aptos_framework = @0x1, creator = @atomic_bridge, moveth = @moveth, client = @0xface, master_minter = @0xbab)]
    fun test_complete_transfer_assets(
        aptos_framework: &signer,
        master_minter: &signer,
        client: &signer,
        creator: &signer,
        moveth: &signer,
    ) acquires BridgeTransferStore, BridgeConfig {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        moveth::init_for_test(moveth);
        let receiver_address = @0xcafe1;
        let initiator = b"0x123"; //In real world this would be an ethereum address
        let recipient = @0xface; 
        let asset = moveth::metadata();

        // the master minter sets client to be a minter
        moveth::add_minter(master_minter, signer::address_of(client));

        //client now mints themselves 100 moveth
        moveth::mint(client, signer::address_of(client), 100);
        assert!(primary_fungible_store::balance(signer::address_of(client), asset) == 100, 0);


        // In this case the moveth_minter (2nd param) is also the creator.
        init_module(creator);

        let bridge_transfer_id = b"transfer1";
        let pre_image = b"secret";
        let hash_lock = keccak256(pre_image); // Compute the hash lock using keccak256
        let time_lock = 3600;
        let amount = 100;

        let result = lock_bridge_transfer_assets(
            creator,
            initiator,
            bridge_transfer_id,
            hash_lock,
            time_lock,
            recipient,
            amount
        );

        assert!(result, 1);

        // Verify that the transfer is stored in pending_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store.pending_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 2);
        assert!(transfer_details.initiator == initiator, 3);
        assert!(transfer_details.amount == amount, 5);
        assert!(transfer_details.hash_lock == hash_lock, 5);

       let pre_image = b"secret"; 
       let msg:vector<u8> = b"secret";
        debug::print(&utf8(msg));

       // Client must be a moveth minter, otherwise this will fail
       complete_bridge_transfer(
           client,
           bridge_transfer_id,
           pre_image 
       );

        debug::print(&utf8(msg));

        // Verify that the transfer is stored in completed_transfers
        let bridge_store = borrow_global<BridgeTransferStore>(signer::address_of(creator));
        let transfer_details: &BridgeTransferDetails = smart_table::borrow(&bridge_store.completed_transfers, bridge_transfer_id);
        assert!(transfer_details.recipient == recipient, 1);
        assert!(transfer_details.amount == amount, 2);
        assert!(transfer_details.hash_lock == hash_lock, 3);
        assert!(transfer_details.initiator == initiator, 4);
    }
    
}
