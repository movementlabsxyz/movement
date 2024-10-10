script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun enable_bridge_feature(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        let enabled_blob: vector<u64> = vector[
            features::get_atomic_bridge_feature()
        ];

        let disabled_blob: vector<u64> = vector[];

        features::change_feature_flags_for_next_epoch(
                framework_signer, 
                enabled_blob, 
                disabled_blob
        );
        features::on_new_epoch(framework_signer);
    }
}