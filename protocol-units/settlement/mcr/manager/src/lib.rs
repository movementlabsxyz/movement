use movement_types::{BlockCommitment, BlockCommitmentEvent};
use tokio_stream::Stream;

mod manager;

pub use manager::Manager as McrSettlementManager;

pub type CommitmentEventStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<BlockCommitmentEvent, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait McrSettlementManagerOperations {

	/// Adds a block commitment to the manager queue.
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error>;

}
