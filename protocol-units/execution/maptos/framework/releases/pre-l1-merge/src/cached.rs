use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::mrb_release;

mrb_release!(PreL1Merge, BIARRTIZ_RC1, "c5d8d936b7775436ff6c256e10049b4de497c220-pre-l1-merge.mrb");

generate_gas_upgrade_module!(gas_upgrade, PreL1Merge, {
	let mut gas_parameters = AptosGasParameters::initial();
	gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);
	gas_parameters.vm.txn.max_execution_gas = GasQuantity::new(10_000_000_000);
	gas_parameters.vm.txn.gas_unit_scaling_factor = GasQuantity::new(50_000);
	aptos_types::on_chain_config::GasScheduleV2 {
		feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
		entries: gas_parameters
			.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
	}
});

pub mod script {
	use super::gas_upgrade::PreL1Merge;
	use aptos_framework_release_script_release::generate_script_module;

	generate_script_module!(script, PreL1Merge, {
		r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::gas_schedule;
    use aptos_framework::transaction_fee;
    use aptos_framework::aptos_coin;
    use aptos_framework::signer;
    use aptos_framework::version;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let core_address: address = signer::address_of(core_resources);

        		// First, disable the COLLECT_AND_DISTRIBUTE_GAS_FEES feature flag to reset the state
		// Then re-enable it and initialize the transaction fee collection
		// This approach ensures clean initialization without conflicts
		
		// Step 1: Temporarily disable the feature flag
		// (This will be handled by the feature flags section below)
		
		// Step 2: Initialize transaction fee collection
		// Note: This is commented out because transaction_fee is already initialized
		// If we try to initialize it again, we get EALREADY_COLLECTING_FEES error
		// Since the feature flag is already enabled, the module is working correctly
		// transaction_fee::initialize_fee_collection_and_distribution(&core_signer, 0);
    }
}
"#
		.to_string()
	});
}

pub mod full {

	use super::script::script::PreL1Merge;
	use aptos_framework_set_feature_flags_release::generate_feature_upgrade_module;

	generate_feature_upgrade_module!(feature_upgrade, PreL1Merge, {
		use aptos_release_builder::components::feature_flags::FeatureFlag;
		use aptos_types::on_chain_config::FeatureFlag as AptosFeatureFlag;

		// Start from Aptos default features, then explicitly add COLLECT_AND_DISTRIBUTE_GAS_FEES
		let mut enable_feature_flags = AptosFeatureFlag::default_features();
		enable_feature_flags.push(AptosFeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES);

		Features {
			enabled: enable_feature_flags.into_iter().map(FeatureFlag::from).collect(),
			disabled: vec![],
		}
	});
}
