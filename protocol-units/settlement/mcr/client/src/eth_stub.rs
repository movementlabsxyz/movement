use crate::{CommitmentStream, McrSettlementClientOperations};
use alloy_network::Ethereum;
use alloy_provider::ProviderBuilder;
use alloy_provider::RootProvider;
use alloy_transport::Transport;
use alloy_transport::TransportError;
use movement_types::Commitment;
use movement_types::Id;
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
use alloy_transport_ws::WsConnect;
use movement_types::BlockCommitment;

#[derive(Error, Debug)]
pub enum McrEthConnectorError {
	#[error(
		"MCR Settlement Tx fail because gaz estimation is to high. Estimated gaz:{0} gaz limit:{1}"
	)]
	GasLimitExceed(u128, u128),
	#[error("MCR Settlement Tx fail because account fund are insuffisant. error:{0}")]
	InsuffisantFund(String),
	#[error("MCR Settlement Tx send fail because :{0}")]
	SendTxError(#[from] alloy_contract::Error),
	#[error("MCR Settlement Tx send fail because of RPC error :{0}")]
	RpcTxError(String),
	#[error("MCR Settlement BlockAccepted event notification error :{0}")]
	EventNotificationError(#[from] alloy_sol_types::Error),
	#[error("MCR Settlement BlockAccepted event notification stream close")]
	EventNotificationStreamClosed,
}

// Codegen from artifact.
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"abi/mcr.json"
);

const MRC_CONTRACT_ADDRESS: &str = "0xBf7c7AE15E23B2E19C7a1e3c36e245A71500e181";
const MAX_TX_SEND_RETRY: usize = 3;

pub struct McrEthSettlementClient<P: Provider<T, Ethereum>, T: Transport + Clone> {
	rpc_provider: P,
	ws_provider: RootProvider<PubSubFrontend>,
	gas_limit: u128,
	_marker: PhantomData<T>,
}

impl<P: Provider<T, Ethereum>, T: Transport + Clone> McrEthSettlementClient<P, T> {
	pub async fn build_with_provider<S>(
		rpc_provider: P,
		ws_url: S,
		gas_limit: u128,
	) -> Result<Self, anyhow::Error>
	where
		S: Into<String>,
	{
		let ws = WsConnect::new(ws_url);

		let ws_provider = ProviderBuilder::new().on_ws(ws).await?;

		Ok(McrEthSettlementClient {
			rpc_provider,
			ws_provider,
			gas_limit,
			_marker: Default::default(),
		})
	}
}

#[async_trait::async_trait]
impl<P: Provider<T, Ethereum>, T: Transport + Clone> McrSettlementClientOperations
	for McrEthSettlementClient<P, T>
{
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		let contract = MCR::new(MRC_CONTRACT_ADDRESS.parse().unwrap(), &self.rpc_provider);
		let call_builder = contract.createBlockCommitment(
			U256::from(block_commitment.height),
			alloy_primitives::FixedBytes(block_commitment.commitment.0[..32].try_into()?),
			alloy_primitives::FixedBytes(block_commitment.block_id.0[..32].try_into()?),
		);
		let MCR::createBlockCommitmentReturn { _0: eth_block_commitment } =
			call_builder.call().await?;
		let base_call_builder = contract.submitBlockCommitment(eth_block_commitment);

		//validate gaz price
		let mut estimate_gas = call_builder.estimate_gas().await?;
		let gas_price = call_builder.provider.get_gas_price().await?;
		let tx_fee_wei = estimate_gas * gas_price;

		println!("estimate_gas:{estimate_gas} gas_price:{gas_price} tx_fee_wei:{tx_fee_wei}");

		if tx_fee_wei > self.gas_limit {
			return Err(McrEthConnectorError::GasLimitExceed(tx_fee_wei, self.gas_limit).into());
		}

		// Sending Tx automatically can lead to erros that depend on the state for Eth.
		// It's convenient to manage some of them automatically to avoid to fail commitment Tx.
		// I define a first one but other should be added depending on the test with mainnet.
		for _ in 0..MAX_TX_SEND_RETRY {
			let call_builder = base_call_builder.clone().gas(estimate_gas);
			//send the Tx and wait for 2 confirmation.
			let pending_tx = match call_builder.send().await {
				Err(alloy_contract::Error::TransportError(TransportError::ErrorResp(payload))) => {
					match payload.code {
						//transaction underpriced
						-32000 => {
							if payload.message.contains("transaction underpriced") {
								//increase gas of 10% and retry
								estimate_gas += (estimate_gas * 10) / 100;
								continue;
							} else if payload.message.contains("insufficient funds") {
								return Err(
									McrEthConnectorError::InsuffisantFund(payload.message).into()
								);
							}
						},
						_ => (),
					}
					return Err(McrEthConnectorError::from(alloy_contract::Error::TransportError(
						TransportError::ErrorResp(payload),
					))
					.into());
				},
				Ok(pending_tx) => pending_tx,
				Err(err) => return Err(McrEthConnectorError::from(err).into()),
			};

			//send the Tx and wait for 2 confirmation.
			let _tx_hash = match pending_tx
				.with_required_confirmations(2)
				.with_timeout(Some(std::time::Duration::from_secs(60)))
				.watch()
				.await
			{
				//			Err(alloy_transport::RpcError::Transport(toto)) => 0,
				Ok(tx_hash) => tx_hash,
				Err(err) => return Err(McrEthConnectorError::RpcTxError(err.to_string()).into()),
			};
			//tx send done don't retry
			break;
		}

		Ok(())
	}

	async fn post_block_commitment_batch(
		&self,
		block_commitment: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error> {
		todo!()
	}

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		//register to contract BlockCommitmentSubmitted event

		let contract = MCR::new(MRC_CONTRACT_ADDRESS.parse().unwrap(), &self.ws_provider);
		let event_filter = contract.BlockAccepted_filter().watch().await?;

		let stream = async_stream::stream! {
			let mut stream = event_filter.into_stream();
			while let Some(event) = stream.next().await {
				let to_yield = event
					.map(|(commitment, _)| BlockCommitment {
						height: 0, //wait PR 65 to be merged
						block_id: Id(commitment.blockHash.0),
						commitment: Commitment(commitment.stateCommitment.0),
					})
					.map_err(|err| McrEthConnectorError::EventNotificationError(err).into());
				yield to_yield;
			}
			yield Err(McrEthConnectorError::EventNotificationStreamClosed.into())
		};
		Ok(Box::pin(stream) as CommitmentStream)
	}

	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error> {
		todo!()
	}

	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error> {
		todo!()
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use alloy_network::EthereumSigner;
	use alloy_provider::ProviderBuilder;
	use alloy_signer_wallet::LocalWallet;
	use movement_types::Commitment;

	#[ignore]
	#[tokio::test]
	async fn test_send_commitment() -> Result<(), anyhow::Error> {
		let signer: LocalWallet = "XXX".parse()?;
		let api_key = "XXX";
		// Build a provider.
		let provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(signer))
			.on_builtin(&format!("https://eth-sepolia.g.alchemy.com/v2/{api_key}"))
			.await?;
		let client = McrEthSettlementClient::build_with_provider(
			provider,
			format!("wss://eth-sepolia.g.alchemy.com/v2/{api_key}"),
			10000000000000000,
		)
		.await
		.unwrap();

		let mut stream = client.stream_block_commitments().await.unwrap();
		// tokio::spawn(async move {
		// 	// Process each socket concurrently.
		// 	while let Some(event) = stream.next().await {
		// 		match event {
		// 			Ok(commitment) => println!("commitment:{commitment:?}"),
		// 			Err(err) => println!("event stream error:{err:?}"),
		// 		}
		// 	}
		// });

		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment::test(),
		};

		let res = client.post_block_commitment(commitment).await;
		println!("result {res:?}",);
		assert!(res.is_ok());

		let event = stream.next().await;
		println!("event:{event:?}");
		//tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
		Ok(())
	}
}
