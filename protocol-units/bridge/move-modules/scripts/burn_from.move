script {
    use aptos_framework::aptos_governance;
    use aptos_framework::native_bridge;

    fun burn_from(core_resources: &signer, account: address, amount: u64) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        native_bridge::burn_from(&framework_signer, account, amount);
    }
} 