use alloy_primitives::Address;
use alloy_primitives::U256;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client as AptosClient, FaucetClient},
	types::{block_info::BlockInfo, LocalAccount},
};
use url::Url;

use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_client::eth_client::MCR;
use suzuka_config::Config as SuzukaConfig;
use tracing::info;

//#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_node_settlement_state() -> anyhow::Result<()> {
	use tracing_subscriber::EnvFilter;
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	info!("Begin test_client_settlement");

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
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

	//1) Start Alice an Bod transfer transactions.
	// Loop on Alice and Bod transfer to produce Tx and block
	tokio::spawn({
		let node_url = node_url.clone();
		let faucet_url = faucet_url.clone();
		async move {
			loop {
				tracing::info!("Run run_alice_bob_tx");
				if let Err(err) = run_alice_bob_tx(&node_url, &faucet_url).await {
					panic!("Alice and Bob transfer Tx fail:{err}");
				}
				let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
			}
		}
	});

	// Wait for some block to be executed.
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

	let client = reqwest::Client::new();

	// Get current node commitment
	let node_commitment_uri = "movement/v1/current_commitment";
	let node_commitment_url = format!("{}{}", node_url, node_commitment_uri);
	let response = client.get(&node_commitment_url).send().await?;
	let node_commitment = response.text().await?;

	println!("node_commitment :{node_commitment:?}");

	let rest_client = AptosClient::new(node_url.clone());
	let cur_blockheight = rest_client.get_ledger_information().await?.state().block_height;

	println!("current cur_blockheight :{cur_blockheight:?}");

	// Init smart contract connection
	let mcr_address: Address = config.mcr.settle.mcr_contract_address.trim().parse()?;

	// Define Signers. Ceremony defines 2 signers (index 1 and 2). The first has 95% of the stakes.
	//
	let validator_private_key = config.mcr.settle.signer_private_key.clone();
	let validator_private_key = validator_private_key.parse::<PrivateKeySigner>()?;
	let validator_address = validator_private_key.address();
	tracing::info!("ICI Test validator signer address:{validator_address}",);
	let provider_client = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(validator_private_key.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let validator_contract = MCR::new(mcr_address, &provider_client);

	// Get the height for this commitment using on-chain commitment.
	let mut commitment_height = 0;
	for index in (cur_blockheight.saturating_sub(5)..=cur_blockheight).rev() {
		let MCR::getValidatorCommitmentAtBlockHeightReturn { _0: onchain_commitment_at_height } =
			validator_contract
				.getValidatorCommitmentAtBlockHeight(U256::from(index), validator_address)
				.call()
				.await?;
		let onchain_commitment_str = hex::encode(&onchain_commitment_at_height.commitment);
		println!("onchain_commitment_str :{onchain_commitment_str:?}");

		if onchain_commitment_str == node_commitment {
			commitment_height = index;
			break;
		}
	}
	assert!(commitment_height != 0, "Commitment not found on the smart contract.");

	// Wait to get the commitment accepted.
	let mut accepted_block_commitment = None;
	let mut nb_try = 0;
	while accepted_block_commitment.is_none() && nb_try < 20 {
		// Try to get an accepted commitment
		let MCR::getAcceptedCommitmentAtBlockHeightReturn {
			_0: get_accepted_commitment_at_block_height,
		} = validator_contract
			.getAcceptedCommitmentAtBlockHeight(U256::from(commitment_height))
			.call()
			.await?;
		//0 height means None.
		if get_accepted_commitment_at_block_height.height != U256::from(0) {
			println!(
				"get_accepted_commitment_at_block_height :{}",
				get_accepted_commitment_at_block_height.height
			);
			accepted_block_commitment = Some(get_accepted_commitment_at_block_height);
			break;
		}
		nb_try += 1;
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	}
	assert!(accepted_block_commitment.is_some(), "Commitment not accepted.");

	// Get current fin state.
	let finview_node_url = format!(
		"{}:{}",
		config.execution_config.maptos_config.fin.fin_rest_listen_hostname,
		config.execution_config.maptos_config.fin.fin_rest_listen_port,
	);
	let fin_state_root_hash_query = "/movement/v1/get-finalized-block-info";
	let fin_state_root_hash_url =
		format!("http://{}{}", finview_node_url, fin_state_root_hash_query);
	println!("block fin_state_root_hash_url:{fin_state_root_hash_url:?}");
	let response = client.get(&fin_state_root_hash_url).send().await?;
	println!("block response:{response:?}");
	let fin_block_info: BlockInfo = response.json().await?;

	// Get block for this height
	let rest_client = AptosClient::new(node_url.clone());
	let block = rest_client.get_block_by_height(commitment_height, false).await?;

	// Compare the block hash with fin_block_info id.
	assert_eq!(
		block.inner().block_hash,
		aptos_sdk::rest_client::aptos_api_types::HashValue(fin_block_info.id()),
		"Fin state doesn't correspond to current block"
	);

	// Wait to get the commitment accepted.
	let mut accepted_block_commitment = None;
	let mut nb_try = 0;
	while accepted_block_commitment.is_none() && nb_try < 20 {
		// Try to get an accepted commitment
		let MCR::getAcceptedCommitmentAtBlockHeightReturn {
			_0: get_accepted_commitment_at_block_height,
		} = validator_contract
			.getAcceptedCommitmentAtBlockHeight(U256::from(commitment_height))
			.call()
			.await?;
		//0 height means None.
		if get_accepted_commitment_at_block_height.height != U256::from(0) {
			accepted_block_commitment = Some(get_accepted_commitment_at_block_height);
			break;
		}
		nb_try += 1;
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	}
	assert!(accepted_block_commitment.is_some(), "Commitment not accepted.");

	Ok(())
}

async fn run_alice_bob_tx(node_url: &Url, faucet_url: &Url) -> anyhow::Result<()> {
	let rest_client = AptosClient::new(node_url.clone());
	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone()); // <:!:section_1a

	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let mut bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	faucet_client.fund(alice.address(), 100_000_000).await?;
	faucet_client.fund(bob.address(), 100_000_000).await?;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	loop {
		// Have Alice send Bob some coins.
		let txn_hash = coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;
		rest_client.wait_for_transaction(&txn_hash).await?;

		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		// Have Bod send Alice some more coins.
		let txn_hash = coin_client.transfer(&mut bob, alice.address(), 1_000, None).await?;
		rest_client.wait_for_transaction(&txn_hash).await?;

		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	}
}
