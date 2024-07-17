
use howzit::Howzit;
use aptos_sdk::{
    rest_client::{AptosBaseUrl, Client, FaucetClient},
    types::LocalAccount,
};
use std::{env, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
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

    let faucet_client = FaucetClient::new_from_rest_client(
        faucet_url.parse()?,   
        rest_client.clone()
    ).with_auth_token(
        token.clone()
    );

    let howzit = Howzit::generate(
        crate_path_buf.join("howzit"),
        rest_client.clone(),
        faucet_client,
    );

    howzit.build_and_publish().await?;

    let (transaction_result_sender, mut transaction_result_receiver) = tokio::sync::mpsc::unbounded_channel::<bool>();

    // fund the accounts in an orderly manner
    let n = 100;
    let k = 20;
    let mut local_accounts = Vec::with_capacity(n);
    for _ in 0..n {
        let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
        {
            let faucet_client = FaucetClient::new_from_rest_client(
                "https://faucet.devnet.suzuka.movementlabs.xyz".parse()?,   
                rest_client.clone()
            ).with_auth_token(
                token.clone()
            );
            faucet_client.fund(
                alice.address(),
                10_000_000_000,
            ).await.context("Failed to fund account")?;
        }
        local_accounts.push(alice);
    }
    let mut futures = Vec::with_capacity(n);
    let start_time = std::time::Instant::now();
    for alice in local_accounts {
        let shared_alice = Arc::new(RwLock::new(alice));

        for _ in 0..k {
            let howzit = howzit.clone();
            let sender = transaction_result_sender.clone();
            let alice = shared_alice.clone();
            futures.push(tokio::spawn(async move {
                    let mut alice = alice.write().await;
                    match howzit.call_probe(&mut *alice).await {
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
    let success_per_second = (success * 2) as f64 / duration.as_secs_f64();
    println!("Successes per second: {}", success_per_second);

    // print failures per second
    let failures_per_second = failures as f64 / duration.as_secs_f64();
    println!("Failures per second: {}", failures_per_second);
    
    Ok(())
}