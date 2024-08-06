use anyhow::Context;
use aptos_sdk::rest_client::{AptosBaseUrl, Client};
use howzit::Howzit;
use std::io::Write;
use std::{env, path::PathBuf};

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
	let token = std::env::var("AUTH_TOKEN").context("AUTH_TOKEN not set")?;
	let rest_url = std::env::var("REST_URL")
		.unwrap_or("https://aptos.devnet.suzuka.movementlabs.xyz".to_string());
	let faucet_url = std::env::var("FAUCET_URL")
		.unwrap_or("https://faucet.devnet.suzuka.movementlabs.xyz".to_string());
	let bench_output_file =
		std::env::var("BENCH_OUTPUT_FILE").unwrap_or("howzit_bench_output.dat".to_string());

	let rest_client_builder = Client::builder(AptosBaseUrl::Custom(rest_url.parse()?))
		.header("Authorization", format!("Bearer {}", token).as_str())?;
	let rest_client = rest_client_builder.build();

	let howzit = Howzit::generate(
		crate_path_buf.join("howzit"),
		rest_client.clone(),
		faucet_url.parse()?,
		token,
	);

	howzit.build_and_publish().await?;

	// fund the accounts in an orderly manner
	let n = 64;
	let l = 3000;
	let k = 128;

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
