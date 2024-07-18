
use howzit::Howzit;
use aptos_sdk::rest_client::{AptosBaseUrl, Client};
use std::{env, path::PathBuf};
use anyhow::Context;   

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {

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

    let (transaction_result_sender, mut transaction_result_receiver) = tokio::sync::mpsc::unbounded_channel::<bool>();

    // fund the accounts in an orderly manner
    let n = 100;
    let k = 20;
    let mut futures = Vec::with_capacity(n);
    let start_time = std::time::Instant::now();
    for _ in 0..n {
        for _ in 0..k {
            let howzit = howzit.clone();
            let sender = transaction_result_sender.clone();
            futures.push(tokio::spawn(async move {
                    match howzit.call_probe().await {
                        Ok(_) => sender.send(true).unwrap(),
                        Err(e) => {
                            eprintln!("Error sending transaction: {:?}", e);
                            sender.send(false).unwrap();
                        }
                    }
                Ok::<(), anyhow::Error>(())
            }));
        }
    }
    drop(transaction_result_sender);

    let counter_task = tokio::spawn(async move {
        let mut successes = 0;
        let mut failures = 0;
        while let Some(result) = transaction_result_receiver.recv().await {
            if result {
                successes += 1;
            } else {
                failures += 1;
            }
       }
       (successes, failures)
    });

    futures::future::try_join_all(futures).await?;
    let end_time = std::time::Instant::now();
    let duration = end_time - start_time;

    println!("Duration: {:?}", duration);
    let (success, failures) = counter_task.await?;

    // print successes per second
    let success_per_second = (success * 3) as f64 / duration.as_secs_f64();
    println!("Successes per second: {}", success_per_second);

    // print failures per second
    let failures_per_second = failures as f64 / duration.as_secs_f64();
    println!("Failures per second: {}", failures_per_second);
    
    Ok(())
}