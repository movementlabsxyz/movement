use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::mrb_release;

mrb_release!(
	BiarritzRc1,
	BIARRTIZ_RC1,
	"4bc8e26943942286f2f010b041b6527336dbf591-biarritz-rc1.mrb"
);

generate_gas_upgrade_module!(gas_upgrade, BiarritzRc1, {
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
	use super::gas_upgrade::BiarritzRc1;
	use aptos_framework_release_script_release::generate_script_module;

	generate_script_module!(script, BiarritzRc1, {
		r#"
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
"#
		.to_string()
	});
}

pub mod full {

	use super::script::script::BiarritzRc1;
	use aptos_framework_set_feature_flags_release::generate_feature_upgrade_module;

	generate_feature_upgrade_module!(feature_upgrade, BiarritzRc1, {
		use aptos_release_builder::components::feature_flags::FeatureFlag;
		use aptos_types::on_chain_config::FeatureFlag as AptosFeatureFlag;

		// start with the default features and append the Governed Gas Pool feature
		let mut aptos_feature_flags = AptosFeatureFlag::default_features();
		aptos_feature_flags.push(AptosFeatureFlag::GOVERNED_GAS_POOL);

		Features {
			enabled: aptos_feature_flags.into_iter().map(FeatureFlag::from).collect(),
			disabled: vec![],
		}
	});
}
