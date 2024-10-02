//use alloy_primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use alloy_primitives::U256;
use anyhow::Context;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client as AptosClient, FaucetClient},
	types::LocalAccount,
};
use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_client::eth_client::Client as McrClient;
use mcr_settlement_client::eth_client::MCR;
use mcr_settlement_client::McrSettlementClientOperations;
use mcr_settlement_config::Config as McrConfig;
use movement_types::block::BlockCommitment;
use movement_types::block::Commitment;
use movement_types::block::Id;
use suzuka_config::Config as SuzukaConfig;
use tracing::info;
use url::Url;

#[tokio::test]
async fn test_node_settlement_state() -> anyhow::Result<()> {
	use tracing_subscriber::EnvFilter;
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	info!("Begin test_client_settlement");

	// Wait the suzuka node is started and has finished its genesis process.
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Init from config
	let godfig: Godfig<SuzukaConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);
	let config: SuzukaConfig = godfig.try_wait_for_ready().await?;

	let rpc_url = config.mcr.eth_rpc_connection_url();

	let connection_host =
		config.execution_config.maptos_config.client.maptos_rest_connection_hostname;
	let connection_port = config.execution_config.maptos_config.client.maptos_rest_connection_port;
	let node_url: Url = format!("http://{}:{}", connection_host, connection_port).parse()?;

	let connection_host =
		config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_hostname;
	let connection_port =
		config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_port;
	let faucet_url: Url = format!("http://{}:{}", connection_host, connection_port).parse()?;

	let mcr_address: Address = config.mcr.settle.mcr_contract_address.trim().parse()?;

	// Start test
	let validator_private_key: PrivateKeySigner =
		config.mcr.settle.signer_private_key.clone().parse()?;
	let validator_address = validator_private_key.address();
	let provider_client = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(validator_private_key.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let validator_mcr = MCR::new(mcr_address, &provider_client);

	let testing_config = config.mcr.testing.as_ref().context("Testing config not defined.")?;
	let mcr_config = McrConfig {
		settle: mcr_settlement_config::common::settlement::Config {
			signer_private_key: testing_config
				.well_known_account_private_keys
				.get(2)
				.context("No well known account")?
				.to_string(),
			..config.mcr.settle.clone()
		},
		..config.mcr.clone()
	};
	let validator2_client = McrClient::build_with_config(&mcr_config).await.unwrap();

	// Send all pending commitment with validator2's account to have all sent commitment accepted.
	let mut last_seen_commitment = U256::from(0);
	for index in 2..10 {
		// we suppose that the Suzuka node doesn't send more then 10 commitments.
		let MCR::getValidatorCommitmentAtBlockHeightReturn {
			_0: get_validator_commitment_at_block_height,
		} = validator_mcr
			.getValidatorCommitmentAtBlockHeight(U256::from(index), validator_address)
			.call()
			.await?;
		//0 height means None.
		if get_validator_commitment_at_block_height.height != U256::from(0) {
			// A commitment has been sent. Send the Validator2's one.
			let commitment = BlockCommitment::new(
				get_validator_commitment_at_block_height.height.try_into()?,
				Id::new(get_validator_commitment_at_block_height.blockId.into()),
				Commitment::new(get_validator_commitment_at_block_height.commitment.into()),
			);
			validator2_client.post_block_commitment(commitment).await?;
			last_seen_commitment = get_validator_commitment_at_block_height.height;
		}
	}

	//Do Alice -> Bob transfer
	let aptos_client = AptosClient::new(node_url.clone());
	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone());

	let coin_client = CoinClient::new(&aptos_client);

	// Create two accounts locally, Alice and Bob.
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng);

	faucet_client.fund(alice.address(), 100_000_000).await?;
	faucet_client.fund(bob.address(), 100_000_000).await?;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	// Have Alice send Bob some coins.
	let txn_hash = coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;
	aptos_client.wait_for_transaction(&txn_hash).await?;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

	// Read Alice and bod account balance that shouldn't be updated on fin state.
	let finview_node_url = format!(
		"http://{}:{}",
		config.execution_config.maptos_config.fin.fin_rest_listen_hostname,
		config.execution_config.maptos_config.fin.fin_rest_listen_port,
	);
	let finwiew_aptos_client = AptosClient::new(finview_node_url.clone().parse()?);
	let fin_view_coin_client = CoinClient::new(&finwiew_aptos_client);
	// It should be in error because the account shouldn't be funded.
	let before_finwiew_alice_balance =
		fin_view_coin_client.get_account_balance(&alice.address()).await;
	assert!(
		before_finwiew_alice_balance.is_err(),
		"Before accept fin Alice balance is not in error."
	);
	let before_finwiew_bob_balance = fin_view_coin_client.get_account_balance(&bob.address()).await;
	assert!(before_finwiew_bob_balance.is_err(), "Before accept fin Bob balance is not in error.");

	let final_alice_balance = coin_client.get_account_balance(&alice.address()).await?;
	let final_bob_balance = coin_client.get_account_balance(&bob.address()).await?;

	// The node should have produced one or several blocks with Alice and Bod Tx.
	// Make these blocks commitment accepted. We suppose there's less than 10 blocks.
	let mut last_seen_height = 0;
	for index in 1..10 {
		// Get the associated commitment.
		let MCR::getValidatorCommitmentAtBlockHeightReturn { _0: onchain_commitment } =
			validator_mcr
				.getValidatorCommitmentAtBlockHeight(
					last_seen_commitment + U256::from(index),
					validator_private_key.address(),
				)
				.call()
				.await?;
		if onchain_commitment.height != U256::from(0) {
			last_seen_height = onchain_commitment.height.try_into()?;
			let commitment = BlockCommitment::new(
				last_seen_height,
				Id::new(onchain_commitment.blockId.into()),
				Commitment::new(onchain_commitment.commitment.into()),
			);
			validator2_client.post_block_commitment(commitment).await?;
		} else {
			break;
		}
	}

	// Wait the accepted commitment to be finalized. Can take more than 10s for all finality state.
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(20000)).await;
	let after_finview_alice_balance = fin_view_coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get final Alice's account balance")?;
	let after_finview_bob_balance = fin_view_coin_client
		.get_account_balance(&bob.address())
		.await
		.context("Failed to get final Bob's account balance")?;

	//Alice and bob balance should have been finalized in finview
	assert_eq!(
		after_finview_alice_balance, final_alice_balance,
		"Final Alice balance different from expected"
	);
	assert_eq!(
		after_finview_bob_balance, final_bob_balance,
		"Final Bob balance different from expected"
	);

	Ok(())
}
