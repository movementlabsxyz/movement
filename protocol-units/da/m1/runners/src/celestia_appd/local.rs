use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
	pub fn new() -> Self {
		Local
	}

	pub async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: m1_da_light_node_util::config::local::Config,
	) -> Result<(), anyhow::Error> {
		// celestia-appd start --grpc.enable --home $CELESTIA_APP_PATH --log_level $LOG_LEVEL

		// get the app path
		let app_path = config.appd.celestia_path.context("Celestia app path not set")?;

		// get the websocket address
		let websocket_hostname = config.appd.celestia_websocket_listen_hostname.clone();
		let websocket_port = config.appd.celestia_websocket_listen_port.clone();
		let websocket_address = format!("{}:{}", websocket_hostname, websocket_port);

		// get the rpc address
		let listen_hostname = config.appd.celestia_rpc_listen_hostname.clone();
		let listen_port = config.appd.celestia_rpc_listen_port.clone();
		let rpc_address = format!("{}:{}", listen_hostname, listen_port);

		commander::run_command(
			"celestia-appd",
			&[
				"start",
				"--address",
				format!("tcp://{}", websocket_address).as_str(),
				"--proxy_app",
				format!("tcp://{}", websocket_address).as_str(),
				"--grpc.enable",
				"--home",
				&app_path,
				"--rpc.laddr",
				format!("tcp://{}", rpc_address).as_str(),
			],
		)
		.await?;

		Ok(())
	}
}
