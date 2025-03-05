script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;
    use aptos_framework::aptos_coin;

    fun main(core_resources: &signer) {

        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        aptos_coin::mint(core_resources, @0xce47cdf75adaf48c9b2abb62133436f7860f6ad4a1bbfd89060b6e58b86417cc, 999998911889923819);

	}
}