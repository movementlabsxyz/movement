pub mod blob;
pub mod client;
pub mod height;
pub mod submit;

pub use blob::CelestiaBlob;
pub use height::CelestiaHeight;

use crate::{
	block::{BlockHeight, SequencerBlock},
	error::DaSequencerError,
};
use movement_types::block;
use std::future::Future;
use tokio::sync::{mpsc, oneshot};

/// Functionality for a connector to an external DA like Celestia for handling blobs with block ids.
pub trait DaSequencerExternalDa {
	/// Send the given block id to the external DA. The block id is not immediately sent but
	/// aggregated in a batch and eventually sent to Celestia.
	fn send_block(
		&self,
		block_id: block::Id,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	/// Get the blob of block ids from the external DA at the given Celestia height.
	fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlob>, DaSequencerError>> + Send;

	/// Synchronize with the Celestia network to resend missing block and retrieve lost block
	/// confirmations. During a crash, blocks batched to be sent to Celestia are lost.
	/// The role of this function is to resend all missing blocks up to the current block height as
	/// batches to Celestia. These block should be buffered until the synchronization is done then
	/// sent after in order. The last committed Celestia height and the current Movement block
	/// height are passed as arguments to the function. The bootstrapping algorithm delays execution
	/// by 12 seconds to allow Celestia to finalize any blobs currently in process.
	/// Then the algorithm requests blobs for the next Celestia height until no blobs are returned.
	/// It sends a "blocks committed" notification for each requested batch of blocks. If no blobs
	/// were requested in the previous step, the algorithm must request the blob at the last
	/// committed Celestia height. The last block in the highest blob determines the finalized
	/// block height in Celestia. All blocks from the finalized block height + 1 to the current
	/// block height (inclusive) are sent to the Celestia client to be batched into blobs and then
	/// sent to Celestia. During the synchronization process, the Celestia client buffers all
	/// incoming blocks from the network. After successfully finishing the synchronization process,
	/// the buffered blocks are batched into blobs and sent to Celestia.
	fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_finalized_celestia_height: Option<CelestiaHeight>,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

/// Notification messages for communication with the execution environment (main loop).
#[derive(Debug)]
pub enum ExternalDaNotification {
	/// Notify that the block has been committed on the Celestia network at the specified height.
	BlocksCommitted { block_ids: Vec<block::Id>, celestia_height: CelestiaHeight },
	/// Request a block at s specified height.
	/// Used during the synchronization to request a block that is missing on the Celestia network.
	RequestBlockAtHeight { height: BlockHeight, callback: oneshot::Sender<Option<SequencerBlock>> },
	/// Request a block for a specified block id.
	/// Used during the synchronization to determine the finalized block height on the Celestia network.
	RequestBlockForId { id: block::Id, callback: oneshot::Sender<Option<SequencerBlock>> },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockSource {
	/// A new block arrived from the Movement network.
	Input,
	/// The synchronization process with the Celestia network resends a block.
	Bootstrap,
}

/// Upstream dependency for the interaction with the Celestia network.
pub trait CelestiaClientOps: Sync + Clone {
	fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlob>, DaSequencerError>> + Send;

	fn send_block(
		&self,
		block_id: block::Id,
		source: BlockSource,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

/// Upstream dependency for the interaction with the block storage.
pub trait BlockOps: Sync + Clone {
	fn notify_blocks_committed(
		&self,
		block_ids: Vec<block::Id>,
		celestia_height: CelestiaHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	fn request_block_at_height(
		&self,
		height: BlockHeight,
	) -> impl Future<Output = Result<SequencerBlock, DaSequencerError>> + Send;

	fn request_block_with_id(
		&self,
		id: block::Id,
	) -> impl Future<Output = Result<SequencerBlock, DaSequencerError>> + Send;
}

#[derive(Clone)]
pub struct BlockProvider {
	notifier: mpsc::Sender<ExternalDaNotification>,
}

impl BlockProvider {
	pub fn new(notifier: mpsc::Sender<ExternalDaNotification>) -> Self {
		Self { notifier }
	}

	async fn notify(&self, notification: ExternalDaNotification) -> Result<(), DaSequencerError> {
		self.notifier.send(notification).await.map_err(|e| {
			DaSequencerError::ChannelError(format!("Broken notifier channel: {}", e))
		})?;
		Ok(())
	}
}

impl BlockOps for BlockProvider {
	async fn notify_blocks_committed(
		&self,
		block_ids: Vec<block::Id>,
		celestia_height: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		self.notify(ExternalDaNotification::BlocksCommitted { block_ids, celestia_height })
			.await
	}

	async fn request_block_at_height(
		&self,
		height: BlockHeight,
	) -> Result<SequencerBlock, DaSequencerError> {
		let (tx, rx) = oneshot::channel();
		self.notify(ExternalDaNotification::RequestBlockAtHeight { height, callback: tx })
			.await?;
		let block = rx.await.map_err(|e| {
			DaSequencerError::ChannelError(format!("Broken notifier channel: {}", e))
		})?;
		let block = block.ok_or(DaSequencerError::BlockRetrieval(format!(
			"Block at height {:?} not found",
			height
		)))?;
		Ok(block)
	}

	async fn request_block_with_id(
		&self,
		id: block::Id,
	) -> Result<SequencerBlock, DaSequencerError> {
		let (tx, rx) = oneshot::channel();
		self.notify(ExternalDaNotification::RequestBlockForId { id, callback: tx })
			.await?;
		let block = rx.await.map_err(|e| {
			DaSequencerError::ChannelError(format!("Broken notifier channel: {}", e))
		})?;
		let block = block
			.ok_or(DaSequencerError::BlockRetrieval(format!("Block for id {:?} not found", id)))?;
		Ok(block)
	}
}

#[derive(Clone)]
pub struct CelestiaExternalDa<B: BlockOps, C: CelestiaClientOps> {
	block_provider: B,
	celestia_client: C,
}

impl<B: BlockOps, C: CelestiaClientOps> CelestiaExternalDa<B, C> {
	#[cfg(not(test))]
	const DELAY_SECONDS_BEFORE_BOOTSTRAPPING: std::time::Duration =
		std::time::Duration::from_secs(12);

	pub fn new(block_provider: B, celestia_client: C) -> Self {
		CelestiaExternalDa { block_provider, celestia_client }
	}
}

impl<C: CelestiaClientOps> CelestiaExternalDa<BlockProvider, C> {
	pub fn with_notifier(
		notifier: mpsc::Sender<ExternalDaNotification>,
		celestia_client: C,
	) -> Self {
		Self::new(BlockProvider::new(notifier), celestia_client)
	}
}

impl<B: BlockOps, C: CelestiaClientOps> DaSequencerExternalDa for CelestiaExternalDa<B, C> {
	async fn send_block(&self, block_id: block::Id) -> Result<(), DaSequencerError> {
		self.celestia_client.send_block(block_id, BlockSource::Input).await
	}

	async fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> Result<Option<CelestiaBlob>, DaSequencerError> {
		self.celestia_client.get_blob_at_height(height).await
	}

	async fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_finalized_celestia_height: Option<CelestiaHeight>,
	) -> Result<(), DaSequencerError> {
		// wait to ensure that no blob is pending in the Celestia network
		#[cfg(not(test))]
		tokio::time::sleep(Self::DELAY_SECONDS_BEFORE_BOOTSTRAPPING).await;

		// Determine that last finalized blob and block height
		let height_from = match last_finalized_celestia_height {
			None => 1, // No blobs have been sent to Celestia yet, sync from the start
			Some(last_finalized_celestia_height) => {
				let mut celestia_height = last_finalized_celestia_height;
				let mut last_block_id = self
					.get_blob_at_height(celestia_height)
					.await?
					.ok_or(DaSequencerError::ExternalDaBootstrap(format!(
						"Celestia has no blob at last known finalized height {}",
						celestia_height
					)))?
					.last_block_id();

				// Increase the Celestia height until the tip is reached
				loop {
					celestia_height += 1;
					match self.get_blob_at_height(celestia_height).await? {
						Some(blob) => {
							last_block_id = blob.last_block_id();

							// The blocks in this blob are not confirmed yet.
							self.block_provider
								.notify_blocks_committed(blob.to_vec(), celestia_height)
								.await?;
						}
						None => break, // The tip is reached
					}
				}
				let finalized_block_id =
					last_block_id.ok_or(DaSequencerError::ExternalDaBootstrap(format!(
						"Celestia's last finalized blob at height {} is empty",
						celestia_height - 1
					)))?;
				let finalized_block =
					self.block_provider.request_block_with_id(finalized_block_id).await?;
				let finalized_height: u64 = finalized_block.height().into();

				finalized_height + 1
			}
		};

		// Send all missing blocks to Celestia up to the current block height
		for height in height_from..=current_block_height.into() {
			let sequencer_block =
				self.block_provider.request_block_at_height(BlockHeight::from(height)).await?;
			self.celestia_client
				.send_block(sequencer_block.id(), BlockSource::Bootstrap)
				.await?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use movement_types::block::{self, Block, BlockMetadata};
	use movement_types::transaction::Transaction;
	use rand::Rng;
	use std::collections::{BTreeSet, HashMap};
	use std::sync::Arc;
	use tokio::sync::RwLock;

	fn into_ids(blocks: &[Block]) -> Vec<block::Id> {
		blocks.iter().map(|block| block.id()).collect()
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum CelestiaClientCalls {
		#[default]
		Noop,
		SendBlock(block::Id, BlockSource),
		GetBlobsAtHeight(CelestiaHeight),
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum BlockProviderCalls {
		#[default]
		Noop,
		NotifyBlocksCommitted(Vec<block::Id>, CelestiaHeight),
		RequestBlockAtHeight(BlockHeight),
		RequestBlockForId(block::Id),
	}

	trait Id {
		fn id(&self) -> block::Id;
	}

	impl Id for Block {
		fn id(&self) -> block::Id {
			self.id()
		}
	}

	impl Id for CelestiaBlob {
		fn id(&self) -> block::Id {
			self.iter().next().copied().unwrap()
		}
	}

	#[derive(Clone)]
	struct StoreMockState<B: Id + Clone, C> {
		height: u64,
		data: HashMap<u64, B>,
		index: HashMap<block::Id, u64>,
		calls: Vec<C>,
	}

	impl<B: Id + Clone, C> StoreMockState<B, C> {
		fn new() -> Self {
			Self {
				height: 0,
				data: Default::default(),
				index: Default::default(),
				calls: Default::default(),
			}
		}

		fn get_height(&self) -> u64 {
			self.height - 1
		}

		fn add(&mut self, value: B) {
			self.index.insert(value.id(), self.height);
			self.data.insert(self.height, value);
			self.height += 1;
		}

		fn get_at_height(&self, height: u64) -> Option<B> {
			self.data.get(&height).cloned()
		}

		fn get_for_id(&self, id: &block::Id) -> Option<(u64, B)> {
			self.index.get(id).and_then(|h| self.data.get(h).map(|b| (*h, b.clone())))
		}

		fn add_call(&mut self, call: C) {
			self.calls.push(call);
		}

		fn into_calls(self) -> Vec<C> {
			self.calls
		}
	}

	impl<B: Id + Clone, C> FromIterator<B> for StoreMockState<B, C> {
		fn from_iter<T: IntoIterator<Item = B>>(iter: T) -> Self {
			let mut state: StoreMockState<B, C> = StoreMockState::new();
			for item in iter {
				state.add(item);
			}
			state
		}
	}

	#[derive(Clone)]
	struct CelestiaMock(Arc<RwLock<StoreMockState<CelestiaBlob, CelestiaClientCalls>>>);

	impl CelestiaMock {
		fn new(init: Vec<CelestiaBlob>) -> Self {
			let state = StoreMockState::from_iter(init);
			Self(Arc::new(RwLock::new(state)))
		}

		async fn into_calls(self) -> Vec<CelestiaClientCalls> {
			let mut state = self.0.write().await;
			let state = std::mem::replace(&mut *state, StoreMockState::new());
			state.into_calls()
		}
	}

	impl CelestiaClientOps for CelestiaMock {
		async fn get_blob_at_height(
			&self,
			height: CelestiaHeight,
		) -> Result<Option<CelestiaBlob>, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::GetBlobsAtHeight(height));
			Ok(state.get_at_height(height.into()))
		}

		async fn send_block(
			&self,
			block_id: block::Id,
			source: BlockSource,
		) -> Result<(), DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::SendBlock(block_id, source));
			state.add(CelestiaBlob::from(vec![block_id]));
			Ok(())
		}
	}

	#[derive(Clone)]
	struct BlockProviderMock(Arc<RwLock<StoreMockState<Block, BlockProviderCalls>>>);

	impl BlockProviderMock {
		fn new(init: Vec<Block>) -> Self {
			let state = StoreMockState::from_iter(init);
			Self(Arc::new(RwLock::new(state)))
		}

		async fn get_height(&self) -> BlockHeight {
			BlockHeight::from(self.0.read().await.get_height())
		}

		async fn into_calls(self) -> Vec<BlockProviderCalls> {
			let mut state = self.0.write().await;
			let state = std::mem::replace(&mut *state, StoreMockState::new());
			state.into_calls()
		}
	}

	impl BlockOps for BlockProviderMock {
		async fn notify_blocks_committed(
			&self,
			block_ids: Vec<block::Id>,
			celestia_height: CelestiaHeight,
		) -> Result<(), DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(BlockProviderCalls::NotifyBlocksCommitted(block_ids, celestia_height));
			Ok(())
		}

		async fn request_block_at_height(
			&self,
			height: BlockHeight,
		) -> Result<SequencerBlock, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(BlockProviderCalls::RequestBlockAtHeight(height));
			let block = state.get_at_height(height.into()).unwrap();
			Ok(SequencerBlock::try_new(height, block)?)
		}

		async fn request_block_with_id(
			&self,
			id: block::Id,
		) -> Result<SequencerBlock, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(BlockProviderCalls::RequestBlockForId(id));
			let (height, block) = state.get_for_id(&id).unwrap();
			Ok(SequencerBlock::try_new(BlockHeight::from(height), block)?)
		}
	}

	fn test_blocks(count: usize) -> Vec<Block> {
		if count == 0 {
			return vec![];
		}

		let mut rng = rand::thread_rng();
		let mut blocks = Vec::with_capacity(count);
		let genesis = Block::default();
		let mut parent: block::Id = genesis.id();
		blocks.push(genesis);

		for _ in 0..count - 1 {
			let tx = rng.gen::<[u8; 32]>();
			let tx = Transaction::new(tx.to_vec(), 0, 0);
			let block =
				Block::new(BlockMetadata::BlockMetadata, parent, BTreeSet::from_iter(vec![tx]));
			parent = block.id();
			blocks.push(block);
		}

		blocks
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_empty() {
		let blocks = vec![];
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = vec![];
		let celestia = CelestiaMock::new(blobs);
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());

		let current_block_height = BlockHeight::from(0);
		let last_finalized_celestia_height = None;

		assert!(da.bootstrap(current_block_height, last_finalized_celestia_height).await.is_ok());
		assert_eq!(celestia.into_calls().await, vec![]);
		assert_eq!(block_provider.into_calls().await, vec![]);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_in_sync() {
		let blocks = test_blocks(3);
		let ids = into_ids(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = ids.iter().map(|id| CelestiaBlob::from(vec![*id])).collect();
		let celestia = CelestiaMock::new(blobs);
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());

		let current_block_height = block_provider.get_height().await;
		let last_finalized_celestia_height = Some(CelestiaHeight::from(2));

		assert!(da.bootstrap(current_block_height, last_finalized_celestia_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetBlobsAtHeight(2.into()),
				CelestiaClientCalls::GetBlobsAtHeight(3.into()),
			],
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![BlockProviderCalls::RequestBlockForId(ids[2])]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_missing_confirmations() {
		let blocks = test_blocks(3);
		let ids = into_ids(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = ids.iter().map(|id| CelestiaBlob::from(vec![*id])).collect();
		let celestia = CelestiaMock::new(blobs);
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());

		let current_block_height = block_provider.get_height().await;
		let last_finalized_celestia_height = Some(CelestiaHeight::from(0));

		assert!(da.bootstrap(current_block_height, last_finalized_celestia_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetBlobsAtHeight(0.into()),
				CelestiaClientCalls::GetBlobsAtHeight(1.into()),
				CelestiaClientCalls::GetBlobsAtHeight(2.into()),
				CelestiaClientCalls::GetBlobsAtHeight(3.into())
			],
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::NotifyBlocksCommitted(vec![ids[1]], 1.into()),
				BlockProviderCalls::NotifyBlocksCommitted(vec![ids[2]], 2.into()),
				BlockProviderCalls::RequestBlockForId(ids[2]),
			]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_one_behind() {
		let blocks = test_blocks(3);
		let ids = into_ids(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs =
			ids.iter().take(ids.len() - 1).map(|id| CelestiaBlob::from(vec![*id])).collect();
		let celestia = CelestiaMock::new(blobs);
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());

		let current_block_height = block_provider.get_height().await;
		let last_finalized_celestia_height = Some(CelestiaHeight::from(0));

		assert!(da.bootstrap(current_block_height, last_finalized_celestia_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetBlobsAtHeight(0.into()),
				CelestiaClientCalls::GetBlobsAtHeight(1.into()),
				CelestiaClientCalls::GetBlobsAtHeight(2.into()),
				CelestiaClientCalls::SendBlock(ids[2], BlockSource::Bootstrap)
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::NotifyBlocksCommitted(vec![ids[1]], 1.into()),
				BlockProviderCalls::RequestBlockForId(ids[1]),
				BlockProviderCalls::RequestBlockAtHeight(2.into())
			]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_from_genesis() {
		let blocks = test_blocks(3);
		let ids = into_ids(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = vec![];
		let celestia = CelestiaMock::new(blobs);
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());

		let current_block_height = block_provider.get_height().await;
		let last_finalized_celestia_height = None;

		assert!(da.bootstrap(current_block_height, last_finalized_celestia_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::SendBlock(ids[1], BlockSource::Bootstrap),
				CelestiaClientCalls::SendBlock(ids[2], BlockSource::Bootstrap)
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::RequestBlockAtHeight(1.into()),
				BlockProviderCalls::RequestBlockAtHeight(2.into()),
			]
		);
	}
}
