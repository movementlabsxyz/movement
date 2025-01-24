script {
    use aptos_framework::aptos_governance;
    use aptos_framework::native_bridge;
    use aptos_framework::system_addresses;

    fun burn_from(core_resources: &signer, account: address, amount: u64) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        native_bridge::burn_from(&framework_signer, account, amount);
    }
} 
