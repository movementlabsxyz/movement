script {
    use aptos_framework::aptos_governance;
    use aptos_framework::gas_schedule;
	use aptos_framework::governed_gas_pool;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

		governed_gas_pool::initialize(framework_signer, b"aptos_framework::governed_gas_pool");

	}
}