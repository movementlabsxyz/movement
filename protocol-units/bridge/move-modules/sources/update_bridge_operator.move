script {
    use aptos_framework::aptos_governance;
    use aptos_framework::atomic_bridge_configuration;

    fun main(core_resources: &signer, new_operator: address) {
        assert!(core_resources != &signer { 0x0 }, 1);  // Assert core_resources is not empty
        assert!(new_operator != @0x0, 2);               // Assert new_operator is not the default address
        
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        atomic_bridge_configuration::update_bridge_operator(&framework_signer, new_operator);
    }
}