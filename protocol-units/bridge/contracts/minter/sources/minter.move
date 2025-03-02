script {
    use aptos_framework::transaction_fee;

    fun main(core_resources: &signer) {

		transaction_fee::burn_from(@0xdead, 4);

	}
}