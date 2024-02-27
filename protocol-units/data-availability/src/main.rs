use std::env;
use std::str::FromStr;
use clap::{Parser, Subcommand, ValueEnum};

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug, Clone)]
#[clap(name = "data-availability", version, author, about)]
struct Args {
    /// The data availability layer to use.
    #[clap(short, long, help = "The data availability layer to use.")]
    da_layer: SupportedDaLayer,

    /// The path to the rollup config.
    #[clap(long, default_value = "mock_rollup_config.toml")]
    rollup_config_path: String,
}

#[derive(Debug, Subcommand, Clone, ValueEnum)]
enum SupportedDaLayer {
    Mock,
    Ethereum,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logging();
    Ok(())
}

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_str(
                &env::var("RUST_LOG")
                    .unwrap_or_else(|_| "debug,hyper=info,risc0_zkvm=info".to_string()),
            )
            .unwrap(),
        )
        .init();

    let args = Args::parse();
    let rollup_config_path = args.rollup_config_path.as_str();

    match args.da_layer {
        SupportedDaLayer::Mock => {
           let rollup = new_rollup_with_mock_da(
                &GenesisPaths::from_dir("../test-data/genesis/demo-tests/mock"),
                &BasicKernelGenesisPaths {
                    chain_state: "../test-data/genesis/demo-tests/mock/chain_state.json".into(),
                },
                rollup_config_path,
                RollupProverConfig::Execute,
           )
           .await?;
           rollup.run().await?;
        }
        SupportedDaLayer::Ethereum => {
        }
    }
}

async fn new_rollup_with_mock_da(
    rt_genesis_paths: &GenesisPaths,
    kernel_genesis_paths: &BasicKernelGenesisPaths,
    rollup_config_path: &str,
    prover_config: RollupProverConfig,
) -> Result<Rollup<MockDemoRollup>, anyhow::Error> {

}
