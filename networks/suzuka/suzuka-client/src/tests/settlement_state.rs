use alloy_network::Ethereum;
use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_primitives::U256;
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
use alloy_transport::Transport;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use std::env;
use std::str::FromStr;
use url::Url;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"../../../protocol-units/settlement/mcr/client/abi/MCR.json"
);

#[tokio::test]
async fn test_node_settlement_state() -> anyhow::Result<()> {
	//1) start Alice an Bod transfer transactions.
	// loop on Alice abd Bod transfer to produce Tx and block
	let loop_jh = tokio::spawn(async move {
		loop {
			if let Err(err) = run_alice_bob_tx().await {
				panic!("Alice and Bob transfer Tx fail:{err}");
			}
		}
	});

	//2) Get smart contract last accepted commitment
	// Inititalize Test variables
	let rpc_port = env::var("MCR_ANVIL_PORT").unwrap();
	let rpc_url = format!("http://localhost:{rpc_port}");
	let ws_url = format!("ws://localhost:{rpc_port}");

	let anvil_address = read_anvil_json_file_address()?;

	//Do SC ceremony init stake calls.
	do_genesis_ceremonial(&anvil_address, &rpc_url).await?;

	let mcr_address = read_mcr_sc_adress()?;
	//Define Signers. Ceremony define 2 signers with half stake each.
	let signer: LocalWallet = anvil_address[1].1.parse()?;
	let signer_addr = signer.address();
	//Build client 1 and send first commitment.
	let provider_client = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer))
		.on_http(rpc_url.parse().unwrap());

	let contract = MCR::new(mcr_address, &provider_client);

	//try to find the last accepted epoch.
	let MCR::getCurrentEpochReturn { _0: current_epoch } =
		contract.getCurrentEpoch().call().await?;

	let mut accepted_block_commitment = None;
	let mut nb_try = 0;
	while accepted_block_commitment.is_none() && nb_try < 100 {
		let MCR::getAcceptedCommitmentAtBlockHeightReturn {
			_0: get_accepted_commitment_at_block_height,
		} = contract
			.getAcceptedCommitmentAtBlockHeight(U256::from(current_epoch))
			.call()
			.await?;
		//0 height mean None.
		if get_accepted_commitment_at_block_height.height != U256::from(0) {
			accepted_block_commitment = Some(get_accepted_commitment_at_block_height);
			break;
		}
		nb_try += 1;
	}

	match accepted_block_commitment {
		Some(block_commitment) => {
			println!("Get an accepted block at heigh: {:?}", block_commitment.height)
		}
		None => println!("Can't find an accepted block commitment."),
	}

	//3) Get Suzuka block at settlement height

	// verify that the block state match the settlement one. Block is FIN.

	Ok(())
}

async fn run_alice_bob_tx() -> anyhow::Result<()> {
	// let _ =
	// 	tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (self.id as u64))).await;

	let suzuka_config = maptos_execution_util::config::Config::try_from_env()?;
	let node_url = Url::from_str(
		format!("http://{}", suzuka_config.aptos_config.aptos_rest_listen_url.as_str()).as_str(),
	)
	.unwrap();

	let faucet_url = Url::from_str(
		format!("http://{}", suzuka_config.aptos_config.aptos_faucet_listen_url.as_str()).as_str(),
	)
	.unwrap();

	// :!:>section_1a
	let rest_client = Client::new(node_url.clone());
	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone()); // <:!:section_1a

	// :!:>section_1b
	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let mut bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	// :!:>section_3
	faucet_client.fund(alice.address(), 100_000_000).await?;
	faucet_client.create_account(bob.address()).await?;

	// Have Alice send Bob some coins.
	let txn_hash = coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;
	rest_client.wait_for_transaction(&txn_hash).await?;

	// Have Bod send Alice some more coins.
	// :!:>section_5
	let txn_hash = coin_client.transfer(&mut bob, alice.address(), 1_000, None).await?;
	// :!:>section_6
	rest_client.wait_for_transaction(&txn_hash).await?;

	Ok(())
}

use serde_json::{from_str, Value};
use std::fs;
fn read_anvil_json_file_address() -> Result<Vec<(String, String)>, anyhow::Error> {
	let anvil_conf_file = env::var("ANVIL_JSON_PATH")?;
	let file_content = fs::read_to_string(anvil_conf_file)?;

	let json_value: Value = from_str(&file_content)?;

	// Extract the available_accounts and private_keys fields
	let available_accounts_iter = json_value["available_accounts"]
		.as_array()
		.expect("available_accounts should be an array")
		.iter()
		.map(|v| v.as_str().map(|s| s.to_string()))
		.flatten();

	let private_keys_iter = json_value["private_keys"]
		.as_array()
		.expect("private_keys should be an array")
		.iter()
		.map(|v| v.as_str().map(|s| s.to_string()))
		.flatten();

	let res = available_accounts_iter
		.zip(private_keys_iter)
		.collect::<Vec<(String, String)>>();
	Ok(res)
}

fn read_mcr_sc_adress() -> Result<Address, anyhow::Error> {
	let file_path = env::var("MCR_SC_ADDRESS_FILE")?;
	let addr_str = fs::read_to_string(file_path)?;
	let addr: Address = addr_str.trim().parse()?;
	Ok(addr)
}

// Do the Genesis ceremony in Rust because if node by forge script,
// it's never done from Rust call.
use alloy_primitives::Bytes;
use alloy_rpc_types::TransactionRequest;
async fn do_genesis_ceremonial(
	anvil_address: &[(String, String)],
	rpc_url: &str,
) -> Result<(), anyhow::Error> {
	let mcr_address = read_mcr_sc_adress()?;
	//Define Signer. Signer1 is the MCRSettelement client
	let signer1: LocalWallet = anvil_address[1].1.parse()?;
	let signer1_addr: Address = anvil_address[1].0.parse()?;
	let signer1_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer1))
		.on_http(rpc_url.parse()?);
	let signer1_contract = MCR::new(mcr_address, &signer1_rpc_provider);

	stake_genesis(
		&signer1_rpc_provider,
		&signer1_contract,
		mcr_address,
		signer1_addr,
		55_000_000_000_000_000_000,
	)
	.await?;

	let signer2: LocalWallet = anvil_address[2].1.parse()?;
	let signer2_addr: Address = anvil_address[2].0.parse()?;
	let signer2_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer2))
		.on_http(rpc_url.parse()?);
	let signer2_contract = MCR::new(mcr_address, &signer2_rpc_provider);

	//init staking
	// Build a transaction to set the values.
	stake_genesis(
		&signer2_rpc_provider,
		&signer2_contract,
		mcr_address,
		signer2_addr,
		54_000_000_000_000_000_000,
	)
	.await?;

	let MCR::hasGenesisCeremonyEndedReturn { _0: has_genesis_ceremony_ended } =
		signer2_contract.hasGenesisCeremonyEnded().call().await?;
	let ceremony: bool = has_genesis_ceremony_ended.try_into().unwrap();
	assert!(ceremony);
	Ok(())
}

async fn stake_genesis<P: Provider<T, Ethereum>, T: Transport + Clone>(
	provider: &P,
	contract: &MCR::MCRInstance<T, &P, Ethereum>,
	contract_address: Address,
	signer: Address,
	amount: u128,
) -> Result<(), anyhow::Error> {
	let stake_genesis_call = contract.stakeGenesis();
	let calldata = stake_genesis_call.calldata().to_owned();
	sendtx_function(provider, calldata, contract_address, signer, amount).await
}
async fn sendtx_function<P: Provider<T, Ethereum>, T: Transport + Clone>(
	provider: &P,
	call_data: Bytes,
	contract_address: Address,
	signer: Address,
	amount: u128,
) -> Result<(), anyhow::Error> {
	let eip1559_fees = provider.estimate_eip1559_fees(None).await?;
	let tx = TransactionRequest::default()
		.from(signer)
		.to(contract_address)
		.value(U256::from(amount))
		.input(call_data.into())
		.max_fee_per_gas(eip1559_fees.max_fee_per_gas)
		.max_priority_fee_per_gas(eip1559_fees.max_priority_fee_per_gas);

	provider.send_transaction(tx).await?.get_receipt().await?;
	Ok(())
}
