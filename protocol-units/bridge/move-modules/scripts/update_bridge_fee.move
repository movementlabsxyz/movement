script {
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_governance;
    use aptos_framework::native_bridge_configuration;

    fun update_bridge_relayer(core_resources: &signer, new_relayer: address) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        native_bridge_configuration::update_bridge_relayer(&framework_signer, new_relayer);
        aptos_account::create_account(@0x00000000000000000000000000face);
    }
} 
