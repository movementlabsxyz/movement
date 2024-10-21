use anyhow::Context;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::Arc;
use suzuka_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use tokio::sync::RwLock;
use url::Url;
use warp::Filter;

static SUZUKA_CONFIG: Lazy<suzuka_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>().unwrap();
	config
});

// :!:>section_1c
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
// <:!:section_1c

pub enum Modes {
	Test,
	Health,
}

impl FromStr for Modes {
	type Err = ();
	fn from_str(input: &str) -> Result<Self, Self::Err> {
		match input {
			"test" => Ok(Modes::Test),
			"health" => Ok(Modes::Health),
			_ => Err(()),
		}
	}
}

async fn test() -> Result<(), anyhow::Error> {
	// :!:>section_1a
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

	// :!:>section_1b
	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	// Print account addresses.
	println!("\n=== Addresses ===");
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());

	// Create the accounts on chain, but only fund Alice.
	// :!:>section_3
	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;
	faucet_client
		.create_account(bob.address())
		.await
		.context("Failed to fund Bob's account")?; // <:!:section_3

	// Print initial balances.
	println!("\n=== Initial Balances ===");
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance")?
	);

	// Have Alice send Bob some coins.
	let txn_hash = coin_client
		.transfer(&mut alice, bob.address(), 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins")?;
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction")?;

	// Print intermediate balances.
	println!("\n=== Intermediate Balances ===");
	// :!:>section_4
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance the second time")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance the second time")?
	); // <:!:section_4

	// Have Alice send Bob some more coins.
	// :!:>section_5
	let txn_hash = coin_client
		.transfer(&mut alice, bob.address(), 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins")?; // <:!:section_5
															 // :!:>section_6
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction")?; // <:!:section_6

	// Print final balances.
	println!("\n=== Final Balances ===");
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance the second time")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance the second time")?
	);

	Ok(())
}

async fn health() -> Result<(), anyhow::Error> {
	// share a status variable between the two threads
	let status = Arc::new(RwLock::new(true));

	// run the tests on a loop updating the status variable in one task
	let status_clone = status.clone();
	let test_task = tokio::spawn(async move {
		loop {
			match test().await {
				Ok(_) => {
					println!("Test passed");
					tokio::time::sleep(std::time::Duration::from_secs(5)).await;
				}
				Err(e) => {
					println!("Test failed: {:?}", e);
					*status_clone.write().await = false;
					tokio::time::sleep(std::time::Duration::from_secs(5)).await;
				}
			}
		}
	});

	// Spawn a small web server that returns the status
	let status_clone = Arc::clone(&status);
	let health_route = warp::path("health").and_then(move || {
		let status_clone = Arc::clone(&status_clone);
		async move {
			let status = status_clone.read().await;
			let status_str = if *status { "ok" } else { "error" };
			Ok::<_, warp::Rejection>(warp::reply::html(status_str))
		}
	});

	let health_task = tokio::spawn(async move {
		warp::serve(health_route)
			.run(([0, 0, 0, 0], 3030)) // Run on 0.0.0.0:3030
			.await;
	});

	// Join the two tasks
	tokio::try_join!(test_task, health_task)?;

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// get the mode
	// todo: this can be replaced with Clap
	let mode = std::env::args().nth(1).unwrap_or("test".to_string());
	let mode = Modes::from_str(&mode).map_err(|_| anyhow::anyhow!("Invalid mode"))?;

	match mode {
		Modes::Test => test().await?,
		Modes::Health => health().await?,
	}

	Ok(())
}
