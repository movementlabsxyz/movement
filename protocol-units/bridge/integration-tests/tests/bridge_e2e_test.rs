use alloy::primitives::keccak256;
use alloy::primitives::{FixedBytes, U256};
use alloy::providers::ProviderBuilder;
use alloy_network::EthereumWallet;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_types::account_address::AccountAddress;
use bridge_config::Config;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::{
	ethereum::{client::EthClient, event_monitoring::EthMonitoring, types::EthAddress},
	movement::{
		client::MovementClient, event_monitoring::MovementMonitoring, utils::MovementAddress,
	},
};
use bridge_service::types::Amount;
use bridge_service::types::AssetType;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;
use bridge_service::types::HashLockPreImage;
use tokio_stream::StreamExt;
use tracing_subscriber::EnvFilter;

async fn start_bridge_local(config: &Config) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
	let one_stream = EthMonitoring::build(&config.eth).await?;
	let one_client = EthClient::new(&config.eth).await?;
	let two_client = MovementClient::new(&config.movement).await?;

	let two_stream = MovementMonitoring::build(&config.movement).await?;

	let jh = tokio::spawn(async move {
		bridge_service::run_bridge(one_client, one_stream, two_client, two_stream)
			.await
			.unwrap()
	});
	Ok(jh)
}

async fn initiate_eth_bridge_transfer(
	config: &Config,
	harness_client: &HarnessEthClient,
	recipient: MovementAddress,
	hash_lock: HashLock,
	amount: Amount,
) -> Result<(), anyhow::Error> {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(HarnessEthClient::get_initiator_private_key(config)))
		.on_builtin(&harness_client.eth_rpc_url)
		.await?;

	let contract = AtomicBridgeInitiator::new(
		harness_client.eth_client.initiator_contract_address(),
		&rpc_provider,
	);

	let initiator_address =
		BridgeAddress(EthAddress(HarnessEthClient::get_initiator_address(config)));

	let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));

	let recipient_bytes: [u8; 32] =
		recipient_address.0.try_into().expect("Recipient address must be 32 bytes");
	let call = contract
		.initiateBridgeTransfer(
			U256::from(amount.weth_value()),
			FixedBytes(recipient_bytes),
			FixedBytes(hash_lock.0),
		)
		.value(U256::from(amount.eth_value()))
		.from(*initiator_address.0);
	let _ = send_transaction(
		call,
		&send_transaction_rules(),
		harness_client.eth_client.config.transaction_send_retries,
		harness_client.eth_client.config.gas_limit,
	)
	.await
	.map_err(|e| BridgeContractError::GenericError(format!("Failed to send transaction: {}", e)))?;
	Ok(())
}

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let (eth_client_harness, mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	// 1) initialize transfer
	// eth_client
	// 	.deposit_weth_and_approve(SetupEthClient::get_initiator_private_key(&anvil), 1)
	// 	.await
	// 	.expect("Failed to deposit WETH");

	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let mov_recipient = MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face"));

	let amount = Amount(AssetType::EthAndWeth((1, 0)));
	initiate_eth_bridge_transfer(&config, &eth_client_harness, mov_recipient, hash_lock, amount)
		.await
		.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;

	Ok(())
}

#[tokio::test]
async fn test_movement_event() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let config = TestHarness::read_bridge_config().await?;

	let mut one_stream = MovementMonitoring::build(&config.movement).await?;

	//listen to event.
	let mut error_counter = 0;
	loop {
		tokio::select! {
			// Wait on chain one events.
			Some(one_event_res) = one_stream.next() =>{
				match one_event_res {
					Ok(one_event) => {
						println!("Receive event {:?}", one_event);
					}
					Err(err) => {
						println!("Receive error {:?}", err);
						error_counter +=1;
						if error_counter > 5 {
							break;
						}
					}
				}
			}
		}
	}

	Ok(())
}
