use movement_types::BlockCommitment;
use tokio_stream::Stream;

#[cfg(test)]
pub mod tests;

#[cfg(feature = "mock")]
mod mock;

#[cfg(feature = "mock")]
pub use mock::McrSettlementClient;

pub mod eth_client;

#[cfg(feature = "eth")]
pub use eth_client::Client as McrSettlementClient;

mod send_eth_transaction;

type CommitmentStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<BlockCommitment, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait McrSettlementClientOperations {
	/// Posts a block commitment to the settlement client.
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error>;

	/// Posts a batch of block commitments to the settlement client.
	async fn post_block_commitment_batch(
		&self,
		block_commitment: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error>;

	/// Streams block commitments from the settlement client.
	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error>;

	/// Gets the accepted commitment at the given height.
	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error>;

	/// Gets the max tolerable block height.
	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error>;
}
