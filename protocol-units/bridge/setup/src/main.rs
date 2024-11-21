mod local;

use bridge_config::Config;
use godfig::{backend::config_file::ConfigFile, Godfig};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// get the config file
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;

	//get suzuka node  maptosconfig
	// let maptos_config =
	// 	dot_movement.try_get_config_from_json::<maptos_execution_util::config::Config>();

	let maptos_config = {
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<maptos_execution_util::config::Config, ConfigFile> =
			Godfig::new(ConfigFile::new(config_file), vec!["maptos_config".to_string()]);
		godfig.try_wait_for_ready().await
	};
	println!("Update bridge config maptos_config:{maptos_config:?}");
	let settlement_config = {
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<mcr_settlement_config::Config, ConfigFile> =
			Godfig::new(ConfigFile::new(config_file), vec!["mcr".to_string()]);
		godfig.try_wait_for_ready().await
	};
	println!("Update bridge config settlement:{maptos_config:?}");

	//Define bridge config path.
	let pathbuff = bridge_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);
	// get a matching godfig object
	let config_file = dot_movement.try_get_or_create_config_file().await?;
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
			let mut config = config.unwrap_or(Config::default());

			// Update config with Movement node conf if present
			if let Ok(maptos_config) = maptos_config {
				println!("Update bridge config with suzuka node config");
				config.movement.mvt_rpc_connection_hostname =
					maptos_config.client.maptos_rest_connection_hostname;
				config.movement.mvt_rpc_connection_port =
					maptos_config.client.maptos_rest_connection_port;
				config.movement.mvt_faucet_connection_hostname =
					maptos_config.client.maptos_faucet_rest_connection_hostname;
				config.movement.mvt_faucet_connection_port =
					maptos_config.client.maptos_faucet_rest_connection_port;

				//update signer with maptos private key
				config.movement.movement_signer_key = maptos_config.chain.maptos_private_key;
			}
			if let Ok(settlement_config) = settlement_config {
				println!("Update bridge config with settlement config");
				config.eth.eth_rpc_connection_protocol =
					settlement_config.eth_connection.eth_rpc_connection_protocol;
				config.eth.eth_rpc_connection_hostname =
					settlement_config.eth_connection.eth_rpc_connection_hostname;
				config.eth.eth_rpc_connection_port =
					settlement_config.eth_connection.eth_rpc_connection_port;

				config.eth.eth_ws_connection_protocol =
					settlement_config.eth_connection.eth_ws_connection_protocol;
				config.eth.eth_ws_connection_hostname =
					settlement_config.eth_connection.eth_ws_connection_hostname;
				config.eth.eth_ws_connection_port =
					settlement_config.eth_connection.eth_ws_connection_port;

				config.eth.eth_chain_id = settlement_config.eth_connection.eth_chain_id;

				//update signer and keys
				//				config.eth.signer_private_key = settlement_config.settle.signer_private_key;
				config.eth.signer_private_key = settlement_config
					.testing
					.as_ref()
					.unwrap()
					.mcr_testing_admin_account_private_key
					.clone();
				config.testing.eth_well_known_account_private_keys = settlement_config
					.testing
					.as_ref()
					.unwrap()
					.well_known_account_private_keys
					.clone();
			}

			//set timelock for e2e test
			config.eth.time_lock_secs = 60; // 1mn for the e2e test.

			// Use custom as movement node in init.
			config.movement.mvt_init_network = "custom".to_string();

			tracing::info!("Bridge Config before setup: {:?}", config);

			let config = bridge_setup::process_compose_setup(config).await?;
			tracing::info!("Bridge Config after setup: {:?}", config);

			Ok(Some(config))
		})
		.await?;

	println!("Bridge setup done.",);
	//Wait indefinitely to keep the Anvil process alive.
	let join_handle: tokio::task::JoinHandle<()> =
		tokio::spawn(async { std::future::pending().await });
	let _ = join_handle.await;
	Ok(())
}
