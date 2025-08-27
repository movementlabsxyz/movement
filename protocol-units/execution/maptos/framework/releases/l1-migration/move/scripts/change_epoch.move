script {
    use aptos_framework::aptos_governance;
//    use aptos_framework::signer;
    use aptos_framework::block;

    fun main(core_resources: &signer, new_interval_us: u64) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        block::update_epoch_interval_microsecs(&core_signer, new_interval_us);
    }
}
