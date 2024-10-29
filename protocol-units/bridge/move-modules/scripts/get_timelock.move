script {
    use aptos_framework::atomic_bridge_store;
    use aptos_framework::ethereum::EthereumAddress;

    /// Retrieves the timelock for a given bridge transfer ID.
    ///
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.
    /// @return The timelock in seconds for the specified bridge transfer ID.
    fun main(bridge_transfer_id: vector<u8>): u64 acquires atomic_bridge_store::SmartTableWrapper {
        let transfer_details = atomic_bridge_store::get_bridge_transfer_details_initiator(bridge_transfer_id);
        transfer_details.time_lock
    }
}

