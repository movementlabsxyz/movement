pub mod cached;

use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
/// Expose the `maptos_framework_release_util` crate for use in the `pre-l1-merge` release.
pub use maptos_framework_release_util;
use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for PreL1Merge.
commit_hash_with_script!(
	PreL1Merge,                                          // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"edafe2e5ed6ce462fa81d08faf5d5008fa836ca2",          // Commit hash
	6,                                                   // Bytecode version
	"pre-l1-merge.mrb",                                  // MRB file name
	"CACHE_PRE_L1_MERGE_FRAMEWORK_RELEASE"               // Cache environment variable
);

generate_gas_upgrade_module!(gas_upgrade, PreL1Merge, {
	let mut gas_parameters = AptosGasParameters::initial();
	gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);
	gas_parameters.vm.txn.max_execution_gas = GasQuantity::new(10_000_000_000);

	aptos_types::on_chain_config::GasScheduleV2 {
		feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
		entries: gas_parameters
			.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
	}
});
