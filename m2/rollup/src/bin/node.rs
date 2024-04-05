//! This binary runs the rollup full node.

use anyhow::Context;
use clap::Parser;
#[cfg(feature = "celestia_da")]
use m2_rollup::celestia_rollup::CelestiaRollup;
#[cfg(feature = "mock_da")]
use m2_rollup::mock_rollup::MockRollup;
use m2_stf::genesis_config::GenesisPaths;
#[cfg(feature = "celestia_da")]
use sov_celestia_adapter::CelestiaConfig;
#[cfg(feature = "mock_da")]
use sov_mock_da::MockDaConfig;
use sov_modules_rollup_blueprint::{Rollup, RollupBlueprint};
use sov_modules_stf_blueprint::kernels::basic::BasicKernelGenesisConfig;
use sov_modules_stf_blueprint::kernels::basic::BasicKernelGenesisPaths;
use sov_stf_runner::RollupProverConfig;
use sov_stf_runner::{from_toml_path, RollupConfig};
use std::str::FromStr;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

// config and genesis for mock da
// #[cfg(feature = "mock_da")]
// const DEFAULT_CONFIG_PATH: &str = "../../rollup_config.toml";
// #[cfg(feature = "mock_da")]
// const DEFAULT_GENESIS_PATH: &str = "../../test-data/genesis/mock/";
// #[cfg(feature = "mock_da")]
// const DEFAULT_KERNEL_GENESIS_PATH: &str = "../../test-data/genesis/mock/chain_state.json";
//
// // config and genesis for local docker celestia
// #[cfg(feature = "celestia_da")]
// const DEFAULT_CONFIG_PATH: &str = "../../celestia_rollup_config.toml";
// #[cfg(feature = "celestia_da")]
// const DEFAULT_GENESIS_PATH: &str = "../../test-data/genesis/celestia/";
// #[cfg(feature = "celestia_da")]
// const DEFAULT_KERNEL_GENESIS_PATH: &str = "../../test-data/genesis/celestia/chain_state.json";

//TODO add default values to the clap arg proc macro
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// The path to the rollup config.
	#[arg(long)]
	rollup_config_path: String,

	/// The path to the genesis config.
	#[arg(long)]
	genesis_paths: String,
	/// The path to the kernel genesis config.
	#[arg(long)]
	kernel_genesis_paths: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initializing logging
	tracing_subscriber::registry()
		.with(fmt::layer())
		//.with(EnvFilter::from_default_env())
		.with(EnvFilter::from_str("info,hyper=info").unwrap())
		.init();

	let args = Args::parse();
	let rollup_config_path = args.rollup_config_path.as_str();

	let genesis_paths = args.genesis_paths.as_str();
	let kernel_genesis_paths = args.kernel_genesis_paths.as_str();

	let prover_config = if option_env!("CI").is_some() {
		Some(RollupProverConfig::Execute)
	} else if let Some(prover) = option_env!("SOV_PROVER_MODE") {
		match prover {
			"simulate" => Some(RollupProverConfig::Simulate),
			"execute" => Some(RollupProverConfig::Execute),
			"prove" => Some(RollupProverConfig::Prove),
			_ => {
				tracing::warn!(
					prover_mode = prover,
					"Unknown sov prover mode, using 'Skip' default"
				);
				Some(RollupProverConfig::Skip)
			},
		}
	} else {
		None
	};

	let rollup = new_rollup(
		&GenesisPaths::from_dir(genesis_paths),
		&BasicKernelGenesisPaths { chain_state: kernel_genesis_paths.into() },
		rollup_config_path,
		prover_config,
	)
	.await?;
	rollup.run().await
}

#[cfg(feature = "mock_da")]
async fn new_rollup(
	rt_genesis_paths: &GenesisPaths,
	kernel_genesis_paths: &BasicKernelGenesisPaths,
	rollup_config_path: &str,
	prover_config: Option<RollupProverConfig>,
) -> Result<Rollup<MockRollup>, anyhow::Error> {
	info!("Reading rollup config from {rollup_config_path:?}");

	let rollup_config: RollupConfig<MockDaConfig> =
		from_toml_path(rollup_config_path).context("Failed to read rollup configuration")?;

	let mock_rollup = MockRollup {};

	let kernel_genesis = BasicKernelGenesisConfig {
		chain_state: serde_json::from_str(
			&std::fs::read_to_string(&kernel_genesis_paths.chain_state)
				.context("Failed to read chain state")?,
		)?,
	};

	mock_rollup
		.create_new_rollup(rt_genesis_paths, kernel_genesis, rollup_config, prover_config)
		.await
}
//
// #[cfg(feature = "celestia_da")]
// async fn new_rollup(
// 	rt_genesis_paths: &GenesisPaths,
// 	kernel_genesis_paths: &BasicKernelGenesisPaths,
// 	rollup_config_path: &str,
// 	prover_config: Option<RollupProverConfig>,
// ) -> Result<Rollup<CelestiaRollup>, anyhow::Error> {
// 	info!("Starting celestia rollup with config {}", rollup_config_path);
//
// 	let rollup_config: RollupConfig<CelestiaConfig> =
// 		from_toml_path(rollup_config_path).context("Failed to read rollup configuration")?;
//
// 	let kernel_genesis = BasicKernelGenesisConfig {
// 		chain_state: serde_json::from_str(
// 			&std::fs::read_to_string(&kernel_genesis_paths.chain_state)
// 				.context("Failed to read chain state")?,
// 		)?,
// 	};
//
// 	let mock_rollup = CelestiaRollup {};
// 	mock_rollup
// 		.create_new_rollup(rt_genesis_paths, kernel_genesis, rollup_config, prover_config)
// 		.await
// }
