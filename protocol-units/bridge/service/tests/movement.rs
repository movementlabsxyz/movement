use anyhow::Result;
use aptos_sdk::{
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use bridge_service::{chains::{bridge_contracts::BridgeContractError, ethereum::{client::EthClient, types::AtomicBridgeInitiator}}, types::BridgeAddress};
//use bridge_service::chains::movement::client::MovementClient;
//AlloyProvider, AtomicBridgeInitiator,
use rand::prelude::*;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command as TokioCommand,
	sync::oneshot,
	task,
};
use url::Url;

#[derive(Clone)]
pub struct SetupMovementClient {
	///The Apotos Rest Client
	pub rest_client: Client,
	///The Apotos Rest Client
	pub faucet_client: Arc<RwLock<FaucetClient>>,
	///The signer account
	pub signer: Arc<LocalAccount>,
}

impl SetupMovementClient {
	pub async fn setup_local_movement_network(
	) -> Result<(Self, tokio::process::Child), anyhow::Error> {
		let (setup_complete_tx, setup_complete_rx) = oneshot::channel();
		let mut child = TokioCommand::new("movement")
			.args(&["node", "run-local-testnet", "--force-restart", "--assume-yes"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?;

		let stdout = child.stdout.take().expect("Failed to capture stdout");
		let stderr = child.stderr.take().expect("Failed to capture stderr");

		task::spawn(async move {
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
							Ok(None) => {
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
							Ok(None) => {
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
		println!("Setup complete message received.");

		let node_connection_url = "http://127.0.0.1:8080".to_string();
		let node_connection_url = Url::from_str(node_connection_url.as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = "http://127.0.0.1:8081".to_string();
		let faucet_url = Url::from_str(faucet_url.as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		Ok((
			SetupMovementClient {
				rest_client,
				faucet_client: faucet_client,
				signer: Arc::new(LocalAccount::generate(&mut rng)),
			},
			child,
		))
	}

	pub async fn deploy_initiator_contract(&mut self) -> Address {}

	pub async fn deploy_counterpart_contract(&mut self) -> Address {}
}

pub async fn deploy_initiator_contract(&mut self) -> Address {
	let eth_client: &mut EthClient = self.eth_client_mut().expect("EthClient not initialized");
	let contract = AtomicBridgeInitiator::deploy(eth_client.rpc_provider())
		.await
		.expect("Failed to deploy AtomicBridgeInitiator");
	eth_client.set_initiator_contract(contract.with_cloned_provider());
	eth_client.initiator_contract_address().expect("Initiator contract not set")
}

pub async fn deploy_weth_contract(&mut self) -> Address {
	let eth_client = self.eth_client_mut().expect("EthClient not initialized");
	let weth = WETH9::deploy(eth_client.rpc_provider()).await.expect("Failed to deploy WETH9");
	eth_client.set_weth_contract(weth.with_cloned_provider());
	eth_client.weth_contract_address().expect("WETH contract not set")
}

pub async fn deploy_init_contracts(&mut self) {
	let _ = self.deploy_initiator_contract().await;
	let weth_address = self.deploy_weth_contract().await;
	self.eth_client()
		.expect("Failed to get EthClient")
		.initialize_initiator_contract(
			BridgeAddress(weth_address),
			BridgeAddress(self.eth_signer_address()),
		)
		.await
		.expect("Failed to initialize contract");
}
