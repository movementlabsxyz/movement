use bridge_config::Config;
use bridge_service::rest::BridgeRest;
use poem::test::TestClient;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_lock_mint() -> Result<(), anyhow::Error> {
	// Get harness
	let (eth_client_harness, mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	// Define bridge config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = bridge_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// Create the REST service, unwrapping the result
	let (l1_health_tx, mut l1_health_rx) = tokio::sync::mpsc::channel(10);
	let (l2_health_tx, mut l2_health_rx) = tokio::sync::mpsc::channel(10);
	let rest_service =
		Arc::new(BridgeRest::new(&godfig.movement, l1_health_tx, l2_health_tx)?);

	// Create the test client with the routes
	let client = TestClient::new(rest_service.create_routes());

	// Get the L2 balance of the bridge relayer account
	let bridge_relayer = Command::new("movement") //--network
	.args(&[
		"move",
		"view",
		"--function-id",
		"0x1::native_bridge::get_bridge_relayer",
		"--rest-url",
		&config.mvt_rpc_connection_url()
	])
	.stdin(Stdio::piped())
	.stdout(Stdio::piped())
	.stderr(Stdio::piped())
	.spawn()
	.expect("Failed to view bridge_relayer");

	let full_url = format!("{}{}{}", &config.mvt_rpc_connection_url(), bridge_relayer, "/resource/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>");
	let bridge_relayer_balance = client.get(full_url).await?.assert_ok().await?;

	// deposit the same balance as the bridge_relayer_balance to the bridge on L1 if it does not surpass  1m tokens (8 decimals)
	// else burn the excess
	let burn_balance = bridge_relayer_balance - 100_000_000_000_000_000;

	if (burn_balance > 0) {

		let burn = Command::new("movement") //--network
		.args(&[
			"move",
			"run",
			"--function-id",
			"0x1::aptos_account::transfer",
			"--args",
			format!("u64:{}", &burn_balance.to_string()).to_str().unwrap(),
			"address:0xdead",
			"--rest-url",
			&config.mvt_rpc_connection_url()
		])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("Failed to burn excess");
	}


	Ok(())
}
