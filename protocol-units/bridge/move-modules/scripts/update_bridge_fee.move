script {
    use aptos_framework::aptos_governance;
    use aptos_framework::native_bridge;

    fun update_bridge_fee(core_resources: &signer, new_fee: u64) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        native_bridge::update_bridge_fee(&framework_signer, new_fee);
    }
} 