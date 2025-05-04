pub mod cached;

use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::commit_hash_with_script;
use maptos_framework_release_util::compiler::Compiler;

// Helper to select compiler based on TEST_FRAMEWORK_REV
pub fn get_compiler_from_env() -> Compiler {
	match std::env::var("TEST_FRAMEWORK_REV") {
		Ok(rev) => Compiler::test(&rev),
		Err(_) => Compiler::movement(),
	}
}

// Example usage of the macro to generate a build script for BiarritzRc1.
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"27397b5835e6a466c06c884a395653c9ff13d1fe",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_BIARRITZ_RC1_FRAMEWORK_RELEASE"               // Cache environment variable for Elsa
);

generate_gas_upgrade_module!(gas_upgrade, BiarritzRc1, {
	let mut gas_parameters = AptosGasParameters::initial();
	gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);
	gas_parameters.vm.txn.max_execution_gas = GasQuantity::new(10_000_000_000);

	aptos_types::on_chain_config::GasScheduleV2 {
		feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
		entries: gas_parameters
			.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
	}
});
