module atomic_bridge::atomic_bridge_initiator {
    use aptos_std::smart_table::{Self, SmartTable};

    struct BridgeTransfer has key, store{
       amount: u64,
       originator: address, 
       recipient: address,
       bytes32: hash_lock,
       timelock: u64,
    }

    struct BridgeTransferStore has key, store {
        bridge_transfers: SmartTable<vector<u8>, BridgeTransfer>,
    }

    entry fun initialize(owner: &signer, moveth_minter: address) {

    }

}