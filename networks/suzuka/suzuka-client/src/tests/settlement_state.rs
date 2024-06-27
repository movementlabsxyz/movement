use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_primitives::U256;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use url::Url;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"../../../protocol-units/settlement/mcr/client/abis/MCRLegacy.json"
);

#[tokio::test]
async fn test_node_settlement_state() -> anyhow::Result<()> {
	//load local env.
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let suzuka_config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	//1) start Alice an Bod transfer transactions.
	// loop on Alice abd Bod transfer to produce Tx and block
	let loop_jh = tokio::spawn({
		let node_url = suzuka_config.execution_config.maptos_config.faucet.get_rest_url()?;
		let faucet_url = suzuka_config.execution_config.maptos_config.faucet.get_faucet_url()?;
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

	//2) Get smart contract last accepted commitment
	// Inititalize Test variables
	let rpc_url = suzuka_config.mcr.rpc_url.ok_or_else(|| {
		anyhow::anyhow!(format!("Anvil rpc Url not defined in config. Aborting."))
	})?;

	let anvil_config = suzuka_config
		.mcr
		.test_local
		.ok_or_else(|| anyhow::anyhow!("Test local anvil configuration not intialized?"))?;

	println!("test anvil_address");

	let mcr_address: Address = suzuka_config.mcr.mcr_contract_address.trim().parse()?;

	//Define Signers. Ceremony define 2 signers (index 0 and 1). The first has 95% of the stakes.
	let signer: LocalWallet = anvil_config.anvil_keys[0].private_key.parse()?;
	let signer_address = signer.address();
	//	let signer_addr = signer.address();
	//Build client 1 and send first commitment.
	let provider_client = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer))
		.on_http(rpc_url.parse().unwrap());

	let contract = MCR::new(mcr_address, &provider_client);

	println!("test contract");

	//try to find the last accepted epoch.
	// let MCR::getCurrentEpochReturn { _0: current_epoch } =
	// 	contract.getCurrentEpoch().call().await?;

	let mut accepted_block_commitment = None;
	let mut nb_try = 0;
	while accepted_block_commitment.is_none() && nb_try < 20 {
		// //FOR TEST
		// let MCR::getCurrentEpochReturn { _0: get_current_epoch } =
		// 	contract.getCurrentEpoch().call().await?;
		// let current_epoch_test: u64 = get_current_epoch.try_into().unwrap();
		// println!("current_epoch_test: {current_epoch_test:?}");

		// let MCR::getMaxTolerableBlockHeightReturn { _0: getMaxTolerableBlockHeight } =
		// 	contract.getMaxTolerableBlockHeight().call().await?;
		// let getMaxTolerableBlockHeight: u64 = getMaxTolerableBlockHeight.try_into().unwrap();
		// println!("getMaxTolerableBlockHeight: {getMaxTolerableBlockHeight:?}");

		// let MCR::getCurrentEpochStakeReturn { _0: get_current_epoch_stake } =
		// 	contract.getCurrentEpochStake(signer_address).call().await?;
		// let data: u128 = get_current_epoch_stake.try_into().unwrap();
		// println!("get_current_epoch_stake: {data}");

		// let MCR::getValidatorCommitmentAtBlockHeightReturn {
		// 	_0: get_validator_commitment_at_block_height,
		// } = contract
		// 	.getValidatorCommitmentAtBlockHeight(U256::from(current_epoch_test), signer_address)
		// 	.call()
		// 	.await?;
		// println!(
		// 	"getValidatorCommitmentAtBlockHeight {current_epoch_test}: {:?}, {:?}, {:?}",
		// 	get_validator_commitment_at_block_height.height,
		// 	get_validator_commitment_at_block_height.commitment,
		// 	get_validator_commitment_at_block_height.blockId,
		// );

		// let MCR::getValidatorCommitmentAtBlockHeightReturn {
		// 	_0: get_validator_commitment_at_block_height,
		// } = contract
		// 	.getValidatorCommitmentAtBlockHeight(U256::from(1), signer_address)
		// 	.call()
		// 	.await?;
		// println!(
		// 	"getValidatorCommitmentAtBlockHeight 1: {:?}, {:?}, {:?}",
		// 	get_validator_commitment_at_block_height.height,
		// 	get_validator_commitment_at_block_height.commitment,
		// 	get_validator_commitment_at_block_height.blockId,
		// );

		// let MCR::getValidatorCommitmentAtBlockHeightReturn {
		// 	_0: get_validator_commitment_at_block_height,
		// } = contract
		// 	.getValidatorCommitmentAtBlockHeight(U256::from(4), signer_address)
		// 	.call()
		// 	.await?;
		// println!(
		// 	"getValidatorCommitmentAtBlockHeight 4: {:?}, {:?}, {:?}",
		// 	get_validator_commitment_at_block_height.height,
		// 	get_validator_commitment_at_block_height.commitment,
		// 	get_validator_commitment_at_block_height.blockId,
		// );
		// //FIn TEST

		//try to get an accepted commitment at height 2
		let MCR::getAcceptedCommitmentAtBlockHeightReturn {
			_0: get_accepted_commitment_at_block_height,
		} = contract.getAcceptedCommitmentAtBlockHeight(U256::from(2)).call().await?;
		//0 height mean None.
		if get_accepted_commitment_at_block_height.height != U256::from(0) {
			accepted_block_commitment = Some(get_accepted_commitment_at_block_height);
			break;
		}
		nb_try += 1;
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
	}

	println!("find accepted block");

	let height = match accepted_block_commitment {
		Some(block_commitment) => {
			println!("Get an accepted block at heigh: {:?}", block_commitment.height);
			block_commitment.height
		}
		None => panic!("Can't find an accepted block commitment."),
	};

	//3) Get Suzuka block at settlement height
	let client = reqwest::Client::new();
	let base_url = "http://0.0.0.0:30832";
	let state_root_hash_query = format!("/movement/v1/state-root-hash/{}", height);
	let state_root_hash_url = format!("{}{}", base_url, state_root_hash_query);
	let response = client.get(&state_root_hash_url).send().await?;
	let state_key = response.text().await?;

	println!("state_key;{state_key:?}",);

	// verify that the block state match the settlement one. Block is FIN.

	Ok(())
}

async fn run_alice_bob_tx(node_url: &Url, faucet_url: &Url) -> anyhow::Result<()> {
	println!("Start alice bob");
	let rest_client = Client::new(node_url.clone());
	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone()); // <:!:section_1a

	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let mut bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2
	println!("before fund alice bob LocalAccount");

	faucet_client.fund(alice.address(), 100_000_000).await?;
	faucet_client.fund(bob.address(), 100_000_000).await?;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	println!("Do transferts 1");
	// Have Alice send Bob some coins.
	let txn_hash = coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;
	rest_client.wait_for_transaction(&txn_hash).await?;

	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
	println!("Do transferts 2");
	// Have Bod send Alice some more coins.
	let txn_hash = coin_client.transfer(&mut bob, alice.address(), 1_000, None).await?;
	rest_client.wait_for_transaction(&txn_hash).await?;

	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	Ok(())
}

// fn load_local_env() -> Result<(), anyhow::Error> {
// 	// Load variables defined in .env file.
// 	let movement_storage_path =
// 		std::env::var("MOVEMENT_BASE_STORAGE_PATH").unwrap_or("".to_string());
// 	let movement_exec_path = std::env::var("EXEC_PATH").unwrap_or("../../..".to_string());
// 	let mut env_file_path = std::path::PathBuf::from(movement_exec_path);
// 	env_file_path.push(movement_storage_path);
// 	env_file_path.push(".env".to_string());
// 	println!("env_file_path:{env_file_path:?}",);
// 	dotenv::from_filename(env_file_path)?;
// 	Ok(())
// }

// use serde_json::{from_str, Value};
// use std::fs;
// fn read_anvil_json_file_address() -> Result<Vec<(String, String)>, anyhow::Error> {
// 	let anvil_conf_file = env::var("ANVIL_JSON_PATH")?;
// 	let file_content = fs::read_to_string(anvil_conf_file)?;

// 	let json_value: Value = from_str(&file_content)?;

// 	// Extract the available_accounts and private_keys fields
// 	let available_accounts_iter = json_value["available_accounts"]
// 		.as_array()
// 		.expect("available_accounts should be an array")
// 		.iter()
// 		.map(|v| v.as_str().map(|s| s.to_string()))
// 		.flatten();

// 	let private_keys_iter = json_value["private_keys"]
// 		.as_array()
// 		.expect("private_keys should be an array")
// 		.iter()
// 		.map(|v| v.as_str().map(|s| s.to_string()))
// 		.flatten();

// 	let res = available_accounts_iter
// 		.zip(private_keys_iter)
// 		.collect::<Vec<(String, String)>>();
// 	Ok(res)
// }

// fn get_mcr_sc_adress() -> Result<Address, anyhow::Error> {
// 	let mcr_address = std::env::var("ETH_MCR_CONTRACT_ADDRESS")?;
// 	println!("mcr_address:{mcr_address:?}",);
// 	let mcr_address: Address = mcr_address.trim().parse()?;

// 	Ok(mcr_address)
// }

// // Do the Genesis ceremony in Rust because if node by forge script,
// // it's never done from Rust call.
// use alloy_primitives::Bytes;
// use alloy_rpc_types::TransactionRequest;
// async fn do_genesis_ceremonial(
// 	mcr_address: Address,
// 	anvil_address: &[mcr_settlement_config::anvil::AnvilAddressEntry],
// 	rpc_url: &str,
// ) -> Result<(), anyhow::Error> {
// 	//Define Signer. Signer1 is the MCRSettelement client
// 	let signer1: LocalWallet = anvil_address[0].private_key.parse()?;
// 	let signer1_addr: Address = anvil_address[0].address.parse()?;
// 	let signer1_rpc_provider = ProviderBuilder::new()
// 		.with_recommended_fillers()
// 		.signer(EthereumSigner::from(signer1))
// 		.on_http(rpc_url.parse()?);
// 	let signer1_contract = MCR::new(mcr_address, &signer1_rpc_provider);

// 	stake_genesis(
// 		&signer1_rpc_provider,
// 		&signer1_contract,
// 		mcr_address,
// 		signer1_addr,
// 		95_000_000_000_000_000_000,
// 	)
// 	.await?;

// 	let signer2: LocalWallet = anvil_address[1].private_key.parse()?;
// 	let signer2_addr: Address = anvil_address[1].address.parse()?;
// 	let signer2_rpc_provider = ProviderBuilder::new()
// 		.with_recommended_fillers()
// 		.signer(EthereumSigner::from(signer2))
// 		.on_http(rpc_url.parse()?);
// 	let signer2_contract = MCR::new(mcr_address, &signer2_rpc_provider);

// 	//init staking
// 	// Build a transaction to set the values.
// 	stake_genesis(
// 		&signer2_rpc_provider,
// 		&signer2_contract,
// 		mcr_address,
// 		signer2_addr,
// 		6_000_000_000_000_000_000,
// 	)
// 	.await?;

// 	let MCR::hasGenesisCeremonyEndedReturn { _0: has_genesis_ceremony_ended } =
// 		signer2_contract.hasGenesisCeremonyEnded().call().await?;
// 	let ceremony: bool = has_genesis_ceremony_ended.try_into().unwrap();
// 	assert!(ceremony);
// 	Ok(())
// }

// async fn stake_genesis<P: Provider<T, Ethereum>, T: Transport + Clone>(
// 	provider: &P,
// 	contract: &MCR::MCRInstance<T, &P, Ethereum>,
// 	contract_address: Address,
// 	signer: Address,
// 	amount: u128,
// ) -> Result<(), anyhow::Error> {
// 	let stake_genesis_call = contract.stakeGenesis();
// 	let calldata = stake_genesis_call.calldata().to_owned();
// 	sendtx_function(provider, calldata, contract_address, signer, amount).await
// }
// async fn sendtx_function<P: Provider<T, Ethereum>, T: Transport + Clone>(
// 	provider: &P,
// 	call_data: Bytes,
// 	contract_address: Address,
// 	signer: Address,
// 	amount: u128,
// ) -> Result<(), anyhow::Error> {
// 	let eip1559_fees = provider.estimate_eip1559_fees(None).await?;
// 	let tx = TransactionRequest::default()
// 		.from(signer)
// 		.to(contract_address)
// 		.value(U256::from(amount))
// 		.input(call_data.into())
// 		.max_fee_per_gas(eip1559_fees.max_fee_per_gas)
// 		.max_priority_fee_per_gas(eip1559_fees.max_priority_fee_per_gas);

// 	provider.send_transaction(tx).await?.get_receipt().await?;
// 	Ok(())
// }
