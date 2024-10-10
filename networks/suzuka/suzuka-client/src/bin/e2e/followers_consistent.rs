use anyhow::Context;
use dot_movement::DotMovement;
use rand::Rng;
use std::str::FromStr;
use std::sync::Arc;
use suzuka_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use tokio::sync::RwLock;
use tracing::info;
use url::Url;

pub fn get_suzuka_config(
	dot_movement: &DotMovement,
) -> Result<suzuka_config::Config, anyhow::Error> {
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;
	Ok(config)
}

pub fn get_node_url(config: &suzuka_config::Config) -> Result<Url, anyhow::Error> {
	let node_connection_address = config
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port =
		config.execution_config.maptos_config.client.maptos_rest_connection_port.clone();

	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);

	Ok(Url::from_str(node_connection_url.as_str())?)
}

pub fn get_faucet_url(config: &suzuka_config::Config) -> Result<Url, anyhow::Error> {
	let faucet_listen_address = config
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = config
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);

	Ok(Url::from_str(faucet_listen_url.as_str())?)
}

pub fn follower_index_to_dot_movement(
	follower_index: u8,
	dot_movement: &DotMovement,
) -> Result<DotMovement, anyhow::Error> {
	// index 0 is default .moevement path
	if follower_index == 0 {
		return Ok(dot_movement.clone());
	}

	// otherwise, modify the path to include the follower index
	let mut follower_dot_movement = dot_movement.clone();
	let path = follower_dot_movement.get_path().to_path_buf();
	// append -follower-{n} to the last component of the path
	let new_path_str = format!("{}-follower-{}", path.display(), follower_index);
	let new_path = std::path::PathBuf::from(new_path_str);
	info!("Follower path: {:?}", new_path);
	follower_dot_movement.set_path(new_path);

	Ok(follower_dot_movement)
}

pub fn get_follower_config(
	follower_index: u8,
	lead_dot_movement: &DotMovement,
) -> Result<(DotMovement, suzuka_config::Config, Client, FaucetClient), anyhow::Error> {
	let follower_dot_movement = follower_index_to_dot_movement(follower_index, lead_dot_movement)?;

	let config = get_suzuka_config(&follower_dot_movement)?;

	let node_url = get_node_url(&config)?;

	let faucet_url = get_faucet_url(&config)?;

	let rest_client = Client::new(node_url.clone());

	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone());

	Ok((follower_dot_movement, config, rest_client, faucet_client))
}

use std::future::Future;

/// Checks whether results from calling different nodes match.
/// Relies on the DOT_MOVEMENT config in each directory.
/// Takes an async closure to which the an appropriate rest client and faucet client are passed.
pub async fn check_matching<T, F, Fut>(
	lead_dot_movement: &DotMovement,
	follower_count: u8,
	mut closure: F,
) -> Result<(), anyhow::Error>
where
	T: Eq + std::fmt::Debug,
	F: FnMut(DotMovement, suzuka_config::Config, Client, FaucetClient) -> Fut,
	Fut: Future<Output = Result<T, anyhow::Error>>,
{
	let mut last_result: Option<T> = None;
	for i in 0..=follower_count {
		// get all of the info
		let (follower_dot_movement, config, rest_client, faucet_client) =
			get_follower_config(i, lead_dot_movement)?;

		// call the closure
		let result = closure(follower_dot_movement, config, rest_client, faucet_client).await?;

		info!("Result from follower {}: {:?}", i, result);

		// compare the result to the last result
		if let Some(last_result) = last_result {
			if result != last_result {
				return Err(anyhow::anyhow!("Results do not match"));
			}
		}

		// update the last result
		last_result = Some(result);
	}

	Ok(())
}

/// Picks one of the nodes to run the closure against at random
pub async fn pick_one<T, F, Fut>(
	lead_dot_movement: &DotMovement,
	follower_count: u8,
	mut closure: F,
) -> Result<T, anyhow::Error>
where
	F: FnMut(DotMovement, suzuka_config::Config, Client, FaucetClient) -> Fut,
	Fut: Future<Output = Result<T, anyhow::Error>>,
{
	let mut rng = rand::thread_rng();
	let i = rng.gen_range(0, follower_count + 1);

	info!("Picking follower {}", i);
	let (follower_dot_movement, config, rest_client, faucet_client) =
		get_follower_config(i as u8, lead_dot_movement)?;

	closure(follower_dot_movement, config, rest_client, faucet_client).await
}

/// Test basic coin transfer functionality.
pub async fn basic_coin_transfers(
	lead_dot_movement: &DotMovement,
	follower_count: u8,
) -> Result<(), anyhow::Error> {
	let alice = Arc::new(RwLock::new(LocalAccount::generate(&mut rand::rngs::OsRng)));
	let bob = Arc::new(RwLock::new(LocalAccount::generate(&mut rand::rngs::OsRng)));

	// Print account addresses.
	info!("\n=== Addresses ===");
	info!("Alice: {}", alice.read().await.address().to_hex_literal());
	info!("Bob: {}", bob.read().await.address().to_hex_literal());

	// Create the accounts on chain, but only fund Alice. Pick one node to do this against for each.
	// Alice
	let alice_clone = alice.clone();
	pick_one(
		lead_dot_movement,
		follower_count,
		move |_dot_movement, _config, _res_client, faucet_client| {
			// Clone `alice` to move it into the async block safely
			let alice = alice_clone.clone();

			async move {
				let alice = alice.write().await;

				faucet_client
					.fund(alice.address(), 100_000_000)
					.await
					.context("Failed to fund Alice's account")?;

				Ok(())
			}
		},
	)
	.await?;

	// Bob
	let bob_clone = bob.clone();
	pick_one(
		lead_dot_movement,
		follower_count,
		move |_dot_movement, _config, _res_client, faucet_client| {
			// Clone `bob` to move it into the async block safely
			let bob = bob_clone.clone();

			async move {
				let bob = bob.write().await;

				faucet_client
					.create_account(bob.address())
					.await
					.context("Failed to fund Bob's account")?;

				Ok(())
			}
		},
	)
	.await?;

	// check all the coin balances are equal
	let alice_clone = alice.clone();
	let bob_clone = bob.clone();
	check_matching(
		lead_dot_movement,
		follower_count,
		move |_dot_movement, _config, rest_client, _faucet_client| {
			let alice = alice_clone.clone();
			let bob = bob_clone.clone();
			async move {
				let coin_client = CoinClient::new(&rest_client);
				let alice_balance = coin_client
					.get_account_balance(&alice.read().await.address())
					.await
					.context("Failed to get Alice's account balance")?;
				let bob_balance = coin_client
					.get_account_balance(&bob.read().await.address())
					.await
					.context("Failed to get Bob's account balance")?;

				Ok((alice_balance, bob_balance))
			}
		},
	)
	.await?;

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	movement_tracing::init_tracing_subscriber();

	// get the lead dot movement from the environment
	let dot_movement = DotMovement::try_from_env()?;

	// get the follower count from the first argument
	let follower_count = std::env::args()
		.nth(1)
		.ok_or_else(|| anyhow::anyhow!("Expected follower count as first argument"))?;
	let follower_count = u8::from_str(follower_count.as_str())?;

	// run basic coin transfers
	basic_coin_transfers(&dot_movement, follower_count).await?;

	Ok(())
}
