use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::signers::local::PrivateKeySigner;
use aptos_sdk::types::LocalAccount;
use bridge_config::common::eth::EthConfig;
use bridge_config::common::movement::MovementConfig;
use bridge_config::common::testing::TestingConfig;
use bridge_config::Config as BridgeConfig;
use rand::prelude::*;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command as TokioCommand;

pub async fn setup(
	mut config: BridgeConfig,
) -> Result<(BridgeConfig, AnvilInstance), anyhow::Error> {
	tracing::info!("Bridge local setup");
	//Eth init: Start anvil.
	let anvil = setup_eth(&mut config.eth, &mut config.testing);
	tracing::info!("Bridge after anvil");

	//By default the setup deosn't start the Movement node.
	Ok((config, anvil))
}

pub fn setup_eth(config: &mut EthConfig, testing_config: &mut TestingConfig) -> AnvilInstance {
	let anvil = Anvil::new().port(config.eth_rpc_connection_port).spawn();
	//update config with Anvil address
	let signer: PrivateKeySigner = anvil.keys()[1].clone().into();
	config.signer_private_key = signer.to_bytes().to_string();
	for key in anvil.keys().iter().skip(2) {
		let privkey: PrivateKeySigner = (key.clone()).into();
		testing_config
			.eth_well_known_account_private_keys
			.push(privkey.to_bytes().to_string());
	}

	anvil
}

pub async fn setup_movement_node(
	config: &mut MovementConfig,
) -> Result<tokio::process::Child, anyhow::Error> {
	//kill existing process if any.
	let kill_cmd = TokioCommand::new("sh")
			.arg("-c")
			.arg("PID=$(ps aux | grep 'movement node run-local-testnet' | grep -v grep | awk '{print $2}' | head -n 1); if [ -n \"$PID\" ]; then kill -9 $PID; fi")
			.output()
			.await?;

	if !kill_cmd.status.success() {
		tracing::info!("Failed to kill running movement process: {:?}", kill_cmd.stderr);
	} else {
		tracing::info!("Movement process killed if it was running.");
	}

	let delete_dir_cmd = TokioCommand::new("sh")
		.arg("-c")
		.arg("if [ -d '.movement/config.yaml' ]; then rm -rf .movement/config.yaml; fi")
		.output()
		.await?;

	if !delete_dir_cmd.status.success() {
		println!("Failed to delete .movement directory: {:?}", delete_dir_cmd.stderr);
	} else {
		println!(".movement directory deleted if it was present.");
	}

	let (setup_complete_tx, setup_complete_rx) = tokio::sync::oneshot::channel();
	let mut child = TokioCommand::new("movement")
		.args(&["node", "run-local-testnet", "--force-restart", "--assume-yes"])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	let stdout = child.stdout.take().expect("Failed to capture stdout");
	let stderr = child.stderr.take().expect("Failed to capture stderr");

	tokio::task::spawn(async move {
		let mut stdout_reader = BufReader::new(stdout).lines();
		let mut stderr_reader = BufReader::new(stderr).lines();

		loop {
			tokio::select! {
				line = stdout_reader.next_line() => {
					match line {
						Ok(Some(line)) => {
							println!("STDOUT: {}", line);
							if line.contains("Setup is complete") {
								println!("Testnet is up and running!");
								let _ = setup_complete_tx.send(());
																return Ok(());
							}
						},
						Ok(_) => {
							return Err(anyhow::anyhow!("Unexpected end of stdout stream"));
						},
						Err(e) => {
							return Err(anyhow::anyhow!("Error reading stdout: {}", e));
						}
					}
				},
				line = stderr_reader.next_line() => {
					match line {
						Ok(Some(line)) => {
							println!("STDERR: {}", line);
							if line.contains("Setup is complete") {
								println!("Testnet is up and running!");
								let _ = setup_complete_tx.send(());
																return Ok(());
							}
						},
						Ok(_) => {
							return Err(anyhow::anyhow!("Unexpected end of stderr stream"));
						}
						Err(e) => {
							return Err(anyhow::anyhow!("Error reading stderr: {}", e));
						}
					}
				}
			}
		}
	});

	setup_complete_rx.await.expect("Failed to receive setup completion signal");
	println!("Movement node startup complete message received.");

	let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
	let signer = LocalAccount::generate(&mut rng);
	config.movement_signer_address = signer.private_key().clone();

	Ok(child)
}
