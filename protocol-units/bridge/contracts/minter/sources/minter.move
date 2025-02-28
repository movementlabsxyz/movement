script {
    use aptos_framework::aptos_governance;
    use aptos_framework::gas_schedule;
	use aptos_framework::governed_gas_pool;

    fun main(core_resources: &signer) {

		governed_gas_pool::deposit_gas_fee(@0xdead, 4);

	}
}