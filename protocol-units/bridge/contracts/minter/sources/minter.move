script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;
    use aptos_framework::aptos_coin;

    fun main(core_resources: &signer) {

        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

		    transaction_fee::burn_from(framework_signer, @0xdead, 4);
	}
}
