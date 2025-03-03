use movement_types::block::{SuperBlockCommitment, SuperBlockCommitmentEvent};
use tokio_stream::Stream;

mod manager;

pub use manager::Manager as McrSettlementManager;

pub type CommitmentEventStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<SuperBlockCommitmentEvent, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait McrSettlementManagerOperations {
	/// Adds a block commitment to the manager queue.
	async fn post_block_commitment(
		&self,
		block_commitment: SuperBlockCommitment,
	) -> Result<(), anyhow::Error>;
}
