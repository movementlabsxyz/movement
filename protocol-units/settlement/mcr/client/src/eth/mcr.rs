use crate::_send_eth_transaction::InsufficentFunds;
use crate::_send_eth_transaction::SendTransactionErrorRule;
use crate::_send_eth_transaction::UnderPriced;
use crate::_send_eth_transaction::VerifyRule;
use crate::{CommitmentStream, McrSettlementClientOperations};
use alloy_network::Ethereum;
use alloy_primitives::Address;
use alloy_provider::fillers::ChainIdFiller;
use alloy_provider::fillers::FillProvider;
use alloy_provider::fillers::GasFiller;
use alloy_provider::fillers::JoinFill;
use alloy_provider::fillers::NonceFiller;
use alloy_provider::fillers::SignerFiller;
use std::array::TryFromSliceError;
use std::str::FromStr;
//use alloy_provider::fillers::TransactionFiller;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_transport::Transport;
use movement_types::{Commitment, Id};
use std::marker::PhantomData;
use tokio_stream::StreamExt;
//use alloy_network::Network;
use alloy_provider::Provider;
use thiserror::Error;
//use alloy_network::EthereumSigner;
use alloy_primitives::U256;
//use alloy_provider::ProviderBuilder;
use alloy_sol_types::sol;
//use alloy_transport_http::Http;
use alloy::pubsub::PubSubFrontend;
use alloy_network::EthereumSigner;
use alloy_signer_wallet::LocalWallet;
use alloy_transport::BoxTransport;
use alloy_transport_ws::WsConnect;
use movement_types::BlockCommitment;
use std::env;

const MRC_CONTRACT_ADDRESS: &str = "0xBf7c7AE15E23B2E19C7a1e3c36e245A71500e181";
const MAX_TRANSACTION_SEND_RETRY: usize = 10;
const DEFAULT_TRANSACTION_GAS_LIMIT: u128 = 10_000_000_000_000_000;

#[derive(Clone, Debug)]
pub struct McrEthSettlementConfig {
	pub mrc_contract_address: String,
	pub gas_limit: u128,
	pub transaction_send_number_retry: usize,
}

impl McrEthSettlementConfig {
	fn get_from_env<T: FromStr>(env_var: &str) -> Result<T, McrEthConnectorError>
	where
		<T as FromStr>::Err: std::fmt::Display,
	{
		env::var(env_var)
			.map_err(|err| {
				McrEthConnectorError::BadlyDefineEnvVariable(format!(
					"{env_var} env var is not defined :{err}"
				))
			})
			.and_then(|v| {
				T::from_str(&v).map_err(|err| {
					McrEthConnectorError::BadlyDefineEnvVariable(format!(
						"Parse error for {env_var} env var:{err}"
					))
				})
			})
	}
	pub fn try_from_env() -> Result<Self, McrEthConnectorError> {
		Ok(McrEthSettlementConfig {
			mrc_contract_address: env::var("MCR_CONTRACT_ADDRESS")
				.unwrap_or(MRC_CONTRACT_ADDRESS.to_string()),
			gas_limit: Self::get_from_env::<u128>("MCR_TRANSACTION_SEND_GAS_LIMIT")?,
			transaction_send_number_retry: Self::get_from_env::<usize>(
				"MCR_TRANSACTION_SEND_NUMBER_RETRY",
			)?,
		})
	}
}

impl Default for McrEthSettlementConfig {
	fn default() -> Self {
		McrEthSettlementConfig {
			mrc_contract_address: MRC_CONTRACT_ADDRESS.to_string(),
			gas_limit: DEFAULT_TRANSACTION_GAS_LIMIT,
			transaction_send_number_retry: MAX_TRANSACTION_SEND_RETRY,
		}
	}
}

#[derive(Error, Debug)]
pub enum McrEthConnectorError {
	#[error(
		"MCR Settlement Transaction fail because gas estimation is to high. Estimated gas:{0} gas limit:{1}"
	)]
	GasLimitExceed(u128, u128),
	#[error("MCR Settlement Transaction fail because account funds are insufficient. error:{0}")]
	InsufficientFunds(String),
	#[error("MCR Settlement Transaction send fail because :{0}")]
	SendTransactionError(#[from] alloy_contract::Error),
	#[error("MCR Settlement Transaction send fail during its execution :{0}")]
	RpcTransactionExecution(String),
	#[error("MCR Settlement BlockAccepted event notification error :{0}")]
	EventNotificationError(#[from] alloy_sol_types::Error),
	#[error("MCR Settlement BlockAccepted event notification stream close")]
	EventNotificationStreamClosed,
	#[error("MCR Settlement Error environment variable:{0}")]
	BadlyDefineEnvVariable(String),
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

pub struct McrEthSettlementClient<P, T> {
	rpc_provider: P,
	signer_address: Address,
	ws_provider: RootProvider<PubSubFrontend>,
	config: McrEthSettlementConfig,
	send_transaction_error_rules: Vec<Box<dyn VerifyRule>>,
	_marker: PhantomData<T>,
}

impl
	McrEthSettlementClient<
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
		BoxTransport,
	>
{
	pub async fn build_with_urls<S2>(
		rpc: &str,
		ws_url: S2,
		signer_private_key: &str,
		config: McrEthSettlementConfig,
	) -> Result<Self, anyhow::Error>
	where
		S2: Into<String>,
	{
		let signer: LocalWallet = signer_private_key.parse()?;
		let signer_address = signer.address();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(signer))
			.on_builtin(rpc)
			.await?;

		McrEthSettlementClient::build_with_provider(rpc_provider, signer_address, ws_url, config)
			.await
	}
}

impl<P: Provider<T, Ethereum> + Clone, T: Transport + Clone> McrEthSettlementClient<P, T> {
	pub async fn build_with_provider<S>(
		rpc_provider: P,
		signer_address: Address,
		ws_url: S,
		config: McrEthSettlementConfig,
	) -> Result<Self, anyhow::Error>
	where
		S: Into<String>,
	{
		let ws = WsConnect::new(ws_url);

		let ws_provider = ProviderBuilder::new().on_ws(ws).await?;

		let rule1: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<UnderPriced>::new());
		let rule2: Box<dyn VerifyRule> =
			Box::new(SendTransactionErrorRule::<InsufficentFunds>::new());
		let send_transaction_error_rules = vec![rule1, rule2];

		Ok(McrEthSettlementClient {
			rpc_provider,
			signer_address,
			ws_provider,
			send_transaction_error_rules,
			config,
			_marker: Default::default(),
		})
	}
}

#[async_trait::async_trait]
impl<P: Provider<T, Ethereum> + Clone, T: Transport + Clone> McrSettlementClientOperations
	for McrEthSettlementClient<P, T>
{
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		let contract = MCR::new(self.config.mrc_contract_address.parse()?, &self.rpc_provider);

		let eth_block_commitment = MCR::BlockCommitment {
			// currently, to simplify the api, we'll say 0 is uncommitted all other numbers are legitimate heights
			height: U256::from(block_commitment.height),
			commitment: alloy_primitives::FixedBytes(block_commitment.commitment.0),
			blockId: alloy_primitives::FixedBytes(block_commitment.block_id.0),
		};

		let call_builder = contract.submitBlockCommitment(eth_block_commitment);

		crate::_send_eth_transaction::send_transaction(
			call_builder,
			&self.send_transaction_error_rules,
			self.config.transaction_send_number_retry,
			self.config.gas_limit,
		)
		.await
	}

	async fn post_block_commitment_batch(
		&self,
		block_commitments: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error> {
		let contract = MCR::new(self.config.mrc_contract_address.parse()?, &self.rpc_provider);

		let eth_block_commitment: Vec<_> = block_commitments
			.into_iter()
			.map(|block_commitment| {
				Ok(MCR::BlockCommitment {
					// currently, to simplify the api, we'll say 0 is uncommitted all other numbers are legitimate heights
					height: U256::from(block_commitment.height),
					commitment: alloy_primitives::FixedBytes(block_commitment.commitment.0),
					blockId: alloy_primitives::FixedBytes(block_commitment.block_id.0),
				})
			})
			.collect::<Result<Vec<_>, TryFromSliceError>>()?;

		let call_builder = contract.submitBatchBlockCommitment(eth_block_commitment);

		crate::_send_eth_transaction::send_transaction(
			call_builder,
			&self.send_transaction_error_rules,
			self.config.transaction_send_number_retry,
			self.config.gas_limit,
		)
		.await
	}

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		//register to contract BlockCommitmentSubmitted event

		let contract = MCR::new(self.config.mrc_contract_address.parse()?, &self.ws_provider);
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
		let contract = MCR::new(self.config.mrc_contract_address.parse()?, &self.ws_provider);
		let MCR::getAcceptedCommitmentAtBlockHeightReturn { _0: commitment } =
			contract.getAcceptedCommitmentAtBlockHeight(U256::from(height)).call().await?;
		let return_height: u64 = commitment.height.try_into()?;
		// Commitment with height 0 mean not found
		Ok((return_height != 0).then_some(BlockCommitment {
			height: commitment.height.try_into()?,
			block_id: Id(commitment.blockId.into()),
			commitment: Commitment(commitment.commitment.into()),
		}))
	}

	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error> {
		let contract = MCR::new(self.config.mrc_contract_address.parse()?, &self.ws_provider);
		let MCR::getMaxTolerableBlockHeightReturn { _0: block_height } =
			contract.getMaxTolerableBlockHeight().call().await?;
		let return_height: u64 = block_height.try_into()?;
		Ok(return_height)
	}
}

#[cfg(test)]
pub mod test {}
