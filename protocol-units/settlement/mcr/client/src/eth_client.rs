use crate::send_eth_transaction::InsufficentFunds;
use crate::send_eth_transaction::SendTransactionErrorRule;
use crate::send_eth_transaction::UnderPriced;
use crate::send_eth_transaction::VerifyRule;
use crate::{CommitmentStream, McrSettlementClientOperations};
use alloy::pubsub::PubSubFrontend;
use alloy_network::Ethereum;
use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_primitives::U256;
use alloy_provider::fillers::ChainIdFiller;
use alloy_provider::fillers::FillProvider;
use alloy_provider::fillers::GasFiller;
use alloy_provider::fillers::JoinFill;
use alloy_provider::fillers::NonceFiller;
use alloy_provider::fillers::SignerFiller;
use alloy_provider::Provider;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
use alloy_transport::BoxTransport;
use alloy_transport_ws::WsConnect;
use mcr_settlement_config::Config;
use movement_types::BlockCommitment;
use movement_types::{Commitment, Id};
use serde_json::Value as JsonValue;
use std::array::TryFromSliceError;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tokio_stream::StreamExt;

#[derive(Error, Debug)]
pub enum McrEthConnectorError {
	#[error(
		"MCR Settlement Transaction fails because gas estimation is too high. Estimated gas:{0} gas limit:{1}"
	)]
	GasLimitExceed(u128, u128),
	#[error("MCR Settlement Transaction fails because account funds are insufficient. error:{0}")]
	InsufficientFunds(String),
	#[error("MCR Settlement Transaction send failed because :{0}")]
	SendTransactionError(#[from] alloy_contract::Error),
	#[error("MCR Settlement Transaction send failed during its execution :{0}")]
	RpcTransactionExecution(String),
	#[error("MCR Settlement BlockAccepted event notification error :{0}")]
	EventNotificationError(#[from] alloy_sol_types::Error),
	#[error("MCR Settlement BlockAccepted event notification stream close")]
	EventNotificationStreamClosed,
}

// Note: we prefer using the ABI because the [`sol!`](alloy_sol_types::sol) macro, when used with smart contract code directly, will not handle inheritance.
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"abis/MCR.json"
);

// Note: we prefer using the ABI because the [`sol!`](alloy_sol_types::sol) macro, when used with smart contract code directly, will not handle inheritance.
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MovementStaking,
	"abis/MovementStaking.json"
);

// Note: we prefer using the ABI because the [`sol!`](alloy_sol_types::sol) macro, when used with smart contract code directly, will not handle inheritance.
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MOVEToken,
	"abis/MOVEToken.json"
);
// When created, kill the pid when dropped.
// Use to kill Anvil process when Suzuka Node ends.
// TODO should be removed by the new config.
struct AnvilKillAtDrop {
	pid: u32,
}

impl Drop for AnvilKillAtDrop {
	fn drop(&mut self) {
		tracing::info!("Killing Anvil process pid:{}", self.pid);
		if let Err(err) = std::process::Command::new("kill").args(&[&self.pid.to_string()]).spawn()
		{
			tracing::info!("warn, an error occurs during Anvil process kill : {err}");
		}
	}
}

pub struct Client<P> {
	rpc_provider: P,
	ws_provider: RootProvider<PubSubFrontend>,
	pub signer_address: Address,
	contract_address: Address,
	send_transaction_error_rules: Vec<Box<dyn VerifyRule>>,
	gas_limit: u64,
	send_transaction_retries: u32,
	kill_anvil_process: Option<AnvilKillAtDrop>,
}

impl
	Client<
		FillProvider<
			JoinFill<
				JoinFill<
					JoinFill<JoinFill<alloy_provider::Identity, GasFiller>, NonceFiller>,
					ChainIdFiller,
				>,
				SignerFiller<EthereumSigner>,
			>,
			RootProvider<BoxTransport>,
			BoxTransport,
			Ethereum,
		>,
	>
{
	pub async fn build_with_config(config: Config) -> Result<Self, anyhow::Error> {
		let signer_private_key = config.signer_private_key.clone();
		let signer: LocalWallet = signer_private_key.parse()?;
		let signer_address = signer.address();
		let contract_address = config.mcr_contract_address.parse()?;
		let rpc_url = config.eth_rpc_connection_url();
		let ws_url = config.eth_ws_connection_url();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(signer))
			.on_builtin(&rpc_url)
			.await?;

		let mut client = Client::build_with_provider(
			rpc_provider,
			ws_url,
			signer_address,
			contract_address,
			config.gas_limit,
			config.transaction_send_retries,
		)
		.await?;
		if let Some(pid) = config.anvil_process_pid {
			client.kill_anvil_process = Some(AnvilKillAtDrop { pid })
		}
		Ok(client)
	}
}

impl<P> Client<P> {
	async fn build_with_provider<S>(
		rpc_provider: P,
		ws_url: S,
		signer_address: Address,
		contract_address: Address,
		gas_limit: u64,
		send_transaction_retries: u32,
	) -> Result<Self, anyhow::Error>
	where
		P: Provider + Clone,
		S: Into<String>,
	{
		let ws = WsConnect::new(ws_url);

		let ws_provider = ProviderBuilder::new().on_ws(ws).await?;

		let rule1: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<UnderPriced>::new());
		let rule2: Box<dyn VerifyRule> =
			Box::new(SendTransactionErrorRule::<InsufficentFunds>::new());
		let send_transaction_error_rules = vec![rule1, rule2];

		Ok(Client {
			rpc_provider,
			ws_provider,
			signer_address,
			contract_address,
			send_transaction_error_rules,
			gas_limit,
			send_transaction_retries,
			kill_anvil_process: None,
		})
	}
}

#[async_trait::async_trait]
impl<P> McrSettlementClientOperations for Client<P>
where
	P: Provider + Clone,
{
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		let contract = MCR::new(self.contract_address, &self.rpc_provider);

		let eth_block_commitment = MCR::BlockCommitment {
			// Currently, to simplify the API, we'll say 0 is uncommitted all other numbers are legitimate heights
			height: U256::from(block_commitment.height),
			commitment: alloy_primitives::FixedBytes(block_commitment.commitment.0),
			blockId: alloy_primitives::FixedBytes(block_commitment.block_id.0),
		};

		let call_builder = contract.submitBlockCommitment(eth_block_commitment);

		crate::send_eth_transaction::send_transaction(
			call_builder,
			&self.send_transaction_error_rules,
			self.send_transaction_retries,
			self.gas_limit as u128,
		)
		.await
	}

	async fn post_block_commitment_batch(
		&self,
		block_commitments: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error> {
		let contract = MCR::new(self.contract_address, &self.rpc_provider);

		let eth_block_commitment: Vec<_> = block_commitments
			.into_iter()
			.map(|block_commitment| {
				Ok(MCR::BlockCommitment {
					// Currently, to simplify the API, we'll say 0 is uncommitted all other numbers are legitimate heights
					height: U256::from(block_commitment.height),
					commitment: alloy_primitives::FixedBytes(block_commitment.commitment.0),
					blockId: alloy_primitives::FixedBytes(block_commitment.block_id.0),
				})
			})
			.collect::<Result<Vec<_>, TryFromSliceError>>()?;

		let call_builder = contract.submitBatchBlockCommitment(eth_block_commitment);

		crate::send_eth_transaction::send_transaction(
			call_builder,
			&self.send_transaction_error_rules,
			self.send_transaction_retries,
			self.gas_limit as u128,
		)
		.await
	}

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		// Register to contract BlockCommitmentSubmitted event

		let contract = MCR::new(self.contract_address, &self.ws_provider);
		let event_filter = contract.BlockAccepted_filter().watch().await?;

		let stream = event_filter.into_stream().map(|event| {
			event
				.and_then(|(commitment, _)| {
					let height = commitment.height.try_into().map_err(
						|err: alloy::primitives::ruint::FromUintError<u64>| {
							alloy_sol_types::Error::Other(err.to_string().into())
						},
					)?;
					Ok(BlockCommitment {
						height,
						block_id: Id(commitment.blockHash.0),
						commitment: Commitment(commitment.stateCommitment.0),
					})
				})
				.map_err(|err| McrEthConnectorError::EventNotificationError(err).into())
		});
		Ok(Box::pin(stream) as CommitmentStream)
	}

	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error> {
		let contract = MCR::new(self.contract_address, &self.ws_provider);
		let MCR::getAcceptedCommitmentAtBlockHeightReturn { _0: commitment } = contract
			.getAcceptedCommitmentAtBlockHeight(U256::from(height))
			.call()
			.await?;
		let return_height: u64 = commitment.height.try_into()?;
		// Commitment with height 0 mean not found
		Ok((return_height != 0).then_some(BlockCommitment {
			height: commitment.height.try_into()?,
			block_id: Id(commitment.blockId.into()),
			commitment: Commitment(commitment.commitment.into()),
		}))
	}

	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error> {
		let contract = MCR::new(self.contract_address, &self.ws_provider);
		let MCR::getMaxTolerableBlockHeightReturn { _0: block_height } =
			contract.getMaxTolerableBlockHeight().call().await?;
		let return_height: u64 = block_height.try_into()?;
		Ok(return_height)
	}
}

pub struct AnvilAddressEntry {
	pub address: String,
	pub private_key: String,
}

/// Read the Anvil config file keys and return all address/private keys.
pub fn read_anvil_json_file_addresses<P: AsRef<Path>>(
	anvil_conf_path: P,
) -> Result<Vec<AnvilAddressEntry>, anyhow::Error> {
	let file_content = fs::read_to_string(anvil_conf_path)?;

	let json_value: JsonValue = serde_json::from_str(&file_content)?;

	// Extract the available_accounts and private_keys fields.
	let available_accounts_iter = json_value["available_accounts"]
		.as_array()
		.expect("Available_accounts should be an array")
		.iter()
		.map(|v| {
			let s = v.as_str().expect("Available_accounts elements should be strings");
			s.to_owned()
		});

	let private_keys_iter = json_value["private_keys"]
		.as_array()
		.expect("Private_keys should be an array")
		.iter()
		.map(|v| {
			let s = v.as_str().expect("Private_keys elements should be strings");
			s.to_owned()
		});

	let res = available_accounts_iter
		.zip(private_keys_iter)
		.map(|(address, private_key)| AnvilAddressEntry { address, private_key })
		.collect::<Vec<_>>();
	Ok(res)
}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod tests {
	use super::*;
	use alloy_primitives::Bytes;
	use alloy_provider::ProviderBuilder;
	use alloy_rpc_types::TransactionRequest;
	use alloy_signer_wallet::LocalWallet;
	use alloy_transport::Transport;

	use super::*;
	use alloy_provider::ProviderBuilder;
	use alloy_signer_wallet::LocalWallet;
	use movement_types::Commitment;

	use anyhow::Context;
	use std::env;
	use std::fs;

	
	fn read_mcr_smart_contract_address() -> Result<Address, anyhow::Error> {
		let file_path = env::var("MCR_SMART_CONTRACT_ADDRESS_FILE")?;
		let addr_str = fs::read_to_string(file_path)?;
		let address: Address = addr_str.trim().parse()?;
		Ok(address)
	}

	fn read_staking_smart_contract_address() -> Result<Address, anyhow::Error> {
		let file_path = env::var("STAKING_SMART_CONTRACT_ADDRESS_FILE")?;
		let addr_str = fs::read_to_string(file_path)?;
		let address: Address = addr_str.trim().parse()?;
		Ok(address)
	}

	fn read_move_token_smart_contract_address() -> Result<Address, anyhow::Error> {
		let file_path = env::var("MOVE_TOKEN_SMART_CONTRACT_ADDRESS_FILE")?;
		let addr_str = fs::read_to_string(file_path)?;
		let address: Address = addr_str.trim().parse()?;
		Ok(address)
	}

	// Do the Genesis ceremony in Rust because if node by forge script,
	// it's never done from Rust call.
	use alloy_primitives::Bytes;
	use alloy_rpc_types::TransactionRequest;

	async fn run_genesis_ceremony(
		anvil_address: &[(String, String)],
		rpc_url: &str,
	) -> Result<(), anyhow::Error> {
		// Get the MCR and Staking contract address
		let mcr_address = read_mcr_smart_contract_address()?;
		let staking_address = read_staking_smart_contract_address()?;
		let move_token_address = read_move_token_smart_contract_address()?;

		// Build alice client for MOVEToken, MCR, and staking
		let alice: LocalWallet = anvil_address[1].1.parse()?;
		let alice_address: Address = anvil_address[1].0.parse()?;
		let alice_rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(alice))
			.on_http(rpc_url.parse()?);
		let alice_mcr = MCR::new(mcr_address, &alice_rpc_provider);
		let alice_staking = MovementStaking::new(staking_address, &alice_rpc_provider);
		let alice_move_token = MOVEToken::new(move_token_address, &alice_rpc_provider);

		// Build bob client for MOVEToken, MCR, and staking
		let bob: LocalWallet = anvil_address[2].1.parse()?;
		let bob_address: Address = anvil_address[2].0.parse()?;
		let bob_rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(bob))
			.on_http(rpc_url.parse()?);
		let bob_mcr = MCR::new(mcr_address, &bob_rpc_provider);
		let bob_staking = MovementStaking::new(staking_address, &bob_rpc_provider);
		let bob_move_token = MOVEToken::new(move_token_address, &bob_rpc_provider);

		// Build the MCR client for staking
		let mcr_signer: LocalWallet = mcr_address.to_string().parse()?;
		let mcr_signer_address = mcr_address;
		let mcr_rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(mcr_signer))
			.on_http(rpc_url.parse()?);
		let mcr_staking = MovementStaking::new(staking_address, &mcr_rpc_provider);

		// alice stakes for mcr
		alice_move_token.approve(mcr_address, U256::from(100)).call().await?;
		alice_staking
			.stake(mcr_address, move_token_address, U256::from(100))
			.call()
			.await?;

		// bob stakes for mcr
		bob_move_token.approve(mcr_address, U256::from(100)).call().await?;
		bob_staking
			.stake(mcr_address, move_token_address, U256::from(100))
			.call()
			.await?;

		// mcr accepts the genesis
		mcr_staking.acceptGenesisCeremony().call().await?;

		Ok(())
	}

	#[tokio::test]
	pub fn test_genesis_ceremony() -> Result<(), anyhow::Error> {
		let anvil_address = read_anvil_json_file_addresses("anvil.json")?;
		let rpc_url = env::var("RPC_URL")?;
		tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()?
			.block_on(run_genesis_ceremony(&anvil_address, &rpc_url))?;
		Ok(())
	}

}
