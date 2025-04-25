use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::mrb_release;

mrb_release!(
	PostL1Merge,
	BIARRTIZ_RC1,
	"d00f5e5ef3179919b3fc8245ac774f8509ed6a3e-biarritz-rc1.mrb"
);

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
	use super::gas_upgrade::PostLwMerge;
	use aptos_framework_release_script_release::generate_script_module;

	generate_script_module!(script, PostL1Merge, {
		r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::gas_schedule;
    use aptos_framework::governed_gas_pool;
    use aptos_framework::aptos_coin;
    use aptos_framework::signer;
    use aptos_framework::version;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let core_address: address = signer::address_of(core_resources);

        // this initialize function is idempotent, already initialized GGP will not error.
        governed_gas_pool::initialize(&core_signer, b"aptos_framework::governed_gas_pool");
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

		// start with the default features and append the Governed Gas Pool feature
		let mut aptos_feature_flags = AptosFeatureFlag::default_features();
		// Note: when testing into the future, you may have to use a different revision of [aptos_types] in this crate's Cargo.toml
		// Or, I suppose you can keep and GOVERNED_GAS_POOL feature flag and a GOVERNED_GAS_POOL_V2 feature flag and just make sure you're disabling the former and enabling the latter. Thereafter, it won't matter what happens to the GOVERNED_GAS_POOL feature flag, i.e., it can be replaced.
		aptos_feature_flags.push(AptosFeatureFlag::GOVERNED_GAS_POOL);

		Features {
			enabled: aptos_feature_flags.into_iter().map(FeatureFlag::from).collect(),
			disabled: vec![],
		}
	});
}
