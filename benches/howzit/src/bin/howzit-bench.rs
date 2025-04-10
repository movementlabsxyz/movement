//use anyhow::Context;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::rest_client::{Client, FaucetClient};
use howzit::Howzit;
use once_cell::sync::Lazy;
use std::io::Write;
use std::{env, path::PathBuf, str::FromStr};
use url::Url;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();
	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);
	Url::from_str(node_connection_url.as_str()).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();
	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let crate_path = env!("CARGO_MANIFEST_DIR");
	let crate_path_buf = PathBuf::from(crate_path);
	let bench_output_file =
		std::env::var("BENCH_OUTPUT_FILE").unwrap_or("howzit_bench_output.dat".to_string());

	let howzit = Howzit::generate(
		crate_path_buf.join("howzit"),
		NODE_URL.clone(),
		FAUCET_URL.clone(),
		None, // For now we are going to run local. A var can be used to set this to a testnet later.
	);

	howzit.build_and_publish().await?;

	// fund the accounts in an orderly manner
	let n = std::env::var("HOWZIT_N").unwrap_or("64".to_string()).parse::<usize>()?;
	let l = std::env::var("HOWZIT_L").unwrap_or("3000".to_string()).parse::<u64>()?;
	let k = std::env::var("HOWZIT_K").unwrap_or("64".to_string()).parse::<u64>()?;

	for epoch in 0..l {
		let mut futures = Vec::with_capacity(n);
		// run the load
		for _ in 0..n {
			let howzit = howzit.clone();
			futures.push(tokio::spawn(async move { howzit.call_transfers(k).await }));
		}

		let results = futures::future::try_join_all(futures).await?;

		// append each result to a file
		let mut file = std::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open(bench_output_file.clone())?;
		for result in results {
			match result {
				Ok(result) => {
					for transaction_result in result {
						file.write_all(
							format!(
								"{:?},{:?},{:?},{:?}\n",
								epoch,
								transaction_result.0,
								transaction_result.1,
								transaction_result.2
							)
							.as_bytes(),
						)?;
					}
				}
				Err(e) => {
					tracing::error!("Error: {:?}", e);
					continue;
				}
			}
		}
	}

	Ok(())
}
