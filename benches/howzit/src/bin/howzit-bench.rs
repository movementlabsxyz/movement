
use howzit::Howzit;
use aptos_sdk::rest_client::{AptosBaseUrl, Client};
use std::{env, path::PathBuf};
use anyhow::Context;   

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
    let rest_url = std::env::var("REST_URL").unwrap_or("https://aptos.devnet.suzuka.movementlabs.xyz".to_string());
    let faucet_url = std::env::var("FAUCET_URL").unwrap_or("https://faucet.devnet.suzuka.movementlabs.xyz".to_string());

    let rest_client_builder = Client::builder(
        AptosBaseUrl::Custom(rest_url.parse()?)
    ).header(
        "Authorization",
        format!("Bearer {}", token).as_str()
    )?;
    let rest_client = rest_client_builder.build();

    let howzit = Howzit::generate(
        crate_path_buf.join("howzit"),
        rest_client.clone(),
        faucet_url.parse()?,  
        token
    );

    howzit.build_and_publish().await?;

    let (transaction_result_sender, mut transaction_result_receiver) = tokio::sync::mpsc::unbounded_channel::<(u64, u64)>();

    // fund the accounts in an orderly manner
    let n = 32;

    let k = 1024;
    let mut futures = Vec::with_capacity(n);
    let start_time = std::time::Instant::now();
    for _ in 0..n {
        let howzit = howzit.clone();
        let sender = transaction_result_sender.clone();
        futures.push(tokio::spawn(async move {
            let (successes, failures) = howzit.call_transfers(k).await?;
            sender.send((successes, failures))?;
            Ok::<(), anyhow::Error>(())
        }));
    }
    drop(transaction_result_sender);

    let counter_task = tokio::spawn(async move {
        let mut successes = 0;
        let mut failures = 0;
        while let Some((run_successes, run_failures)) = transaction_result_receiver.recv().await {
            successes += run_successes;
            failures += run_failures;
       }
       (successes, failures)
    });

    futures::future::try_join_all(futures).await?;
    let end_time = std::time::Instant::now();
    let duration = end_time - start_time;

    println!("Duration: {:?}", duration);
    let (success, failures) = counter_task.await?;

    // print successes per second
    let success_per_second = success as f64 / duration.as_secs_f64();
    println!("Successes per second: {}", success_per_second);

    // print failures per second
    let failures_per_second = failures as f64 / duration.as_secs_f64();
    println!("Failures per second: {}", failures_per_second);
    
    Ok(())
}