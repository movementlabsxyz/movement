use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_primitives::U256;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client as AptosClient, FaucetClient},
	types::{block_info::BlockInfo, LocalAccount},
};
use url::Url;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"../../../protocol-units/settlement/mcr/client/abis/MCRLegacy.json"
);

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_node_settlement_state() -> anyhow::Result<()> {
	//load local env.
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let suzuka_config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	let node_url = suzuka_config.execution_config.maptos_config.client.get_rest_url()?;
	let faucet_url = suzuka_config.execution_config.maptos_config.client.get_faucet_url()?;

	//1) start Alice an Bod transfer transactions.
	// loop on Alice abd Bod transfer to produce Tx and block
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

	let client = reqwest::Client::new();

	// Get current node commitment
	let node_commitment_uri = "movement/v1/current_commitment";
	let node_commitment_url = format!("{}{}", node_url, node_commitment_uri);
	let response = client.get(&node_commitment_url).send().await?;
	let node_commitment = response.text().await?;

	let rest_client = AptosClient::new(node_url.clone());
	let cur_blockheight = rest_client.get_ledger_information().await?.state().block_height;

	// Init smart contract connection
	let mcr_address: Address = suzuka_config.mcr.mcr_contract_address.trim().parse()?;
	let rpc_url = suzuka_config.mcr.rpc_url.ok_or_else(|| {
		anyhow::anyhow!(format!("Anvil rpc Url not defined in config. Aborting."))
	})?;
	let anvil_config = suzuka_config
		.mcr
		.test_local
		.ok_or_else(|| anyhow::anyhow!("Test local anvil configuration not intialized?"))?;

	// Define Signers. Ceremony define 2 signers (index 0 and 1). The first has 95% of the stakes.
	let signer: LocalWallet = anvil_config.anvil_keys[0].private_key.parse()?;
	let signer_address = signer.address();
	let provider_client = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer))
		.on_http(rpc_url.parse().unwrap());
	let contract = MCR::new(mcr_address, &provider_client);

	// Get the height for this commitment using onchain commitment.
	let mut commitment_height = 0;
	for index in (cur_blockheight.saturating_sub(5)..=cur_blockheight).rev() {
		let MCR::getValidatorCommitmentAtBlockHeightReturn { _0: onchain_commitment_at_height } =
			contract
				.getValidatorCommitmentAtBlockHeight(U256::from(index), signer_address)
				.call()
				.await?;
		let onchain_commitment_str = hex::encode(&onchain_commitment_at_height.commitment);

		if onchain_commitment_str == node_commitment {
			commitment_height = index;
			break;
		}
	}
	assert!(commitment_height != 0, "Commitment not found on the smart contract.");

	// Get current fin state.
	let finview_node_url = format!(
		"{}:{}",
		suzuka_config.execution_config.maptos_config.fin.fin_rest_listen_hostname,
		suzuka_config.execution_config.maptos_config.fin.fin_rest_listen_port,
	);
	let fin_state_root_hash_query = "/movement/v1/get-finalized-block-info";
	let fin_state_root_hash_url =
		format!("http://{}{}", finview_node_url, fin_state_root_hash_query);
	let response = client.get(&fin_state_root_hash_url).send().await?;
	let fin_block_info: BlockInfo = response.json().await?;

	//get block for this height
	let rest_client = AptosClient::new(node_url.clone());
	let block = rest_client.get_block_by_height(commitment_height, false).await?;

	// Compare the block hash with fin_block_info id.
	//	let block: Block = serde_json::from_str(block.inner())?;
	assert_eq!(
		block.inner().block_hash,
		aptos_sdk::rest_client::aptos_api_types::HashValue(fin_block_info.id()),
		"Fin state doesn't correspond to current block"
	);

	// Wait to get the commitment accepted.
	let mut accepted_block_commitment = None;
	let mut nb_try = 0;
	while accepted_block_commitment.is_none() && nb_try < 20 {
		//try to get an accepted commitment at height 2
		let MCR::getAcceptedCommitmentAtBlockHeightReturn {
			_0: get_accepted_commitment_at_block_height,
		} = contract
			.getAcceptedCommitmentAtBlockHeight(U256::from(commitment_height))
			.call()
			.await?;
		//0 height mean None.
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
	// Have Alice send Bob some coins.
	let txn_hash = coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;
	rest_client.wait_for_transaction(&txn_hash).await?;

	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	// Have Bod send Alice some more coins.
	let txn_hash = coin_client.transfer(&mut bob, alice.address(), 1_000, None).await?;
	rest_client.wait_for_transaction(&txn_hash).await?;

	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	Ok(())
}
