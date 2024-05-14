use movement_types::BlockCommitment;
use tokio_stream::Stream;

#[cfg(feature = "stub")]
pub mod stub;

#[cfg(feature = "stub")]
pub use stub::*;


type CommitmentStream = std::pin::Pin<Box<dyn Stream<Item = Result<BlockCommitment, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait McrSettlementClientOperations {

    async fn post_block_commitment(&self, block_commitment : BlockCommitment) -> Result<(), anyhow::Error>;

    async fn post_block_commitment_batch(&self, block_commitment: Vec<BlockCommitment>) -> Result<(), anyhow::Error>;

    async fn stream_block_commitments(&self) -> Result<
        CommitmentStream,
        anyhow::Error
    >;

    async fn get_commitment_at_height(&self, height : u64) -> Result<Option<BlockCommitment>, anyhow::Error>;

}