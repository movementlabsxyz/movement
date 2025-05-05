pub mod cached;

use aptos_framework_upgrade_gas_release::generate_gas_upgrade_module;
use maptos_framework_release_util::commit_hash_with_script;
use maptos_framework_release_util::compiler::Compiler;

pub fn get_compiler_from_env() -> Compiler {
	match std::env::var("TEST_FRAMEWORK_REV") {
		Ok(rev) => {
			// Convert String to &'static str to satisfy Compiler::test
			let static_rev: &'static str = Box::leak(rev.into_boxed_str());
			Compiler::test(static_rev)
		}
		Err(_) => Compiler::movement(),
	}
}

// Example usage of the macro to generate a build script for PreL1Merge.
commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"dd038ea10e667484d71bf657ae6caaa222156dcf",          // Commit hash
	6,                                                   // Bytecode version
	"post-l1-merge.mrb",                                 // MRB file name
	"CACHE_POST_L1_MERGE_FRAMEWORK_RELEASE"              // Cache environment variable
);

generate_gas_upgrade_module!(gas_upgrade, PostL1Merge, {
	let mut gas_parameters = AptosGasParameters::initial();
	gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);
	gas_parameters.vm.txn.max_execution_gas = GasQuantity::new(10_000_000_000);

	aptos_types::on_chain_config::GasScheduleV2 {
		feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
		entries: gas_parameters
			.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
	}
});
