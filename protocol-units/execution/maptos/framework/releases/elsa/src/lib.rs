pub mod cached;

use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
commit_hash_with_script!(
	Elsa,                                                // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"9dfc8e7a3d622597dfd81cc4ba480a5377f87a41",          // Commit hash
	6,                                                   // Bytecode version
	"elsa.mrb",                                          // MRB file name
	"CACHE_ELSA_FRAMEWORK_RELEASE"                       // Cache environment variable for Elsa
);

generate_gas_upgrade_module!(gas_upgrade, Elsa, {
	let mut gas_parameters = AptosGasParameters::initial();
	gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);
	gas_parameters.vm.txn.max_execution_gas = GasQuantity::new(10_000_000_000);

	aptos_types::on_chain_config::GasScheduleV2 {
		feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
		entries: gas_parameters
			.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
	}
});
