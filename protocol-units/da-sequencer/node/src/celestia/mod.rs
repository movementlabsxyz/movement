use crate::block::{BlockHeight, SequencerBlock, SequencerBlockDigest};
use crate::error::DaSequencerError;
use std::future::Future;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

mod client;
mod height;
mod submit;

pub mod blob;
pub mod mock;

pub use blob::CelestiaBlobData;
pub use height::CelestiaHeight;

/// Functions to implement to save block digest in an external DA like Celestia
pub trait DaSequencerExternalDa {
	/// send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// Until the client can send it to celestia.
	fn send_block(
		&self,
		block: SequencerBlockDigest,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	/// Get the blob from celestia at the given height.
	fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlobData>, DaSequencerError>> + Send;

	/// Bootstrap the Celestia client to recover from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recover this missing block to all block of the network are sent to celestia.
	/// The basic algorithm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// The missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_finalized_celestia_height: Option<CelestiaHeight>,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

/// Message, use to notify CelestiaClient activities.
#[derive(Debug)]
pub enum ExternalDaNotification {
	/// Notify that the block at specified height has been commited on celestia network
	BlockCommitted(BlockHeight, CelestiaHeight),
	/// Ask to send the block at specified height to the Celestia client.
	/// Use during bootstrap to request block that are missing on Celestia network.
	RequestBlock { height: BlockHeight, callback: oneshot::Sender<Option<SequencerBlock>> },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BlockSource {
	/// The block has arrived on the DA service.
	Input,
	/// The block has been recovered in bootstrap.
	Bootstrap,
}

pub trait CelestiaClientOps: Sync + Clone {
	fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlobData>, DaSequencerError>> + Send;

	fn send_block(
		&self,
		block: SequencerBlockDigest,
		source: BlockSource,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

pub trait BlockOps: Sync + Clone {
	fn notify_block_commited(
		&self,
		block_height: BlockHeight,
		celestia_height: CelestiaHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	fn request_block(
		&self,
		height: BlockHeight,
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
	async fn notify_block_commited(
		&self,
		block_height: BlockHeight,
		celestia_height: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		self.notify(ExternalDaNotification::BlockCommitted(block_height, celestia_height))
			.await
	}

	async fn request_block(&self, height: BlockHeight) -> Result<SequencerBlock, DaSequencerError> {
		let (tx, rx) = oneshot::channel();
		self.notify(ExternalDaNotification::RequestBlock { height, callback: tx })
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
}

const DELAY_SECONDS_BEFORE_BOOTSTRAPPING: Duration = Duration::from_secs(12);

#[derive(Clone)]
pub struct CelestiaExternalDa<B: BlockOps, C: CelestiaClientOps> {
	block_provider: B,
	celestia_client: C,
}

impl<B: BlockOps, C: CelestiaClientOps> CelestiaExternalDa<B, C> {
	/// Create the Celestia client and all async process to manage celestia access.
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
	/// Send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// until the client can send it to celestia.
	async fn send_block(&self, block: SequencerBlockDigest) -> Result<(), DaSequencerError> {
		self.celestia_client.send_block(block, BlockSource::Input).await
	}

	/// Get the blob from celestia at the given height.
	async fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> Result<Option<CelestiaBlobData>, DaSequencerError> {
		self.celestia_client.get_blob_at_height(height).await
	}

	/// Bootstrap the Celestia client to recover from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recover this missing block to all block of the network are sent to celestia.
	/// The basic algorithm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// the missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	async fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_finalized_celestia_height: Option<CelestiaHeight>,
	) -> Result<(), DaSequencerError> {
		// wait to ensure that no blob is pending in the Celestia network
		#[cfg(not(test))]
		tokio::time::sleep(DELAY_SECONDS_BEFORE_BOOTSTRAPPING).await;

		// Determine that last finalized blob and block height
		let height_from = match last_finalized_celestia_height {
			None => 1, // No blobs have been sent to Celestia yet, sync from the start
			Some(last_finalized_celestia_height) => {
				let mut celestia_height = last_finalized_celestia_height;
				let mut finalized_blob = self.get_blob_at_height(celestia_height).await?.ok_or(
					DaSequencerError::ExternalDaBootstrap(format!(
						"Celestia has no blob at last known finalized height {}",
						celestia_height
					)),
				)?;

				// Increase the Celestia height until the tip is reached
				loop {
					celestia_height += 1;
					match self.get_blob_at_height(celestia_height).await? {
						Some(blob) => {
							// The blocks in this blob are not confirmed yet.
							for block in blob.iter() {
								self.block_provider
									.notify_block_commited(block.height, celestia_height)
									.await?;
							}
							finalized_blob = blob;
						}
						None => break, // The tip is reached
					}
				}
				let finalized_height: u64 = finalized_blob
					.last_block_height()
					.ok_or(DaSequencerError::ExternalDaBootstrap(format!(
						"Celestia's last finalized blob at height {} is empty",
						celestia_height - 1
					)))?
					.into();

				finalized_height + 1
			}
		};

		// Send all missing blocks to Celestia up to the current block height
		for height in height_from..=current_block_height.into() {
			let block = self.block_provider.request_block(BlockHeight::from(height)).await?;
			self.celestia_client
				.send_block(block.get_block_digest(), BlockSource::Bootstrap)
				.await?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use movement_types::block::{Block, BlockMetadata, Id};
	use movement_types::transaction::Transaction;
	use rand::Rng;
	use std::collections::{BTreeSet, HashMap};
	use std::sync::Arc;
	use tokio::sync::RwLock;

	fn digest(block: &Block, block_height: BlockHeight) -> SequencerBlockDigest {
		SequencerBlockDigest::new(block_height, Id::new(*block.id().as_bytes()))
	}

	fn into_digests(blocks: &[Block]) -> Vec<SequencerBlockDigest> {
		blocks
			.iter()
			.enumerate()
			.map(|(height, block)| digest(block, BlockHeight::from(height as u64)))
			.collect()
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum CelestiaClientCalls {
		#[default]
		Noop,
		SendBlock(SequencerBlockDigest, BlockSource),
		GetBlobsAtHeight(CelestiaHeight),
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum BlockProviderCalls {
		#[default]
		Noop,
		NotifyBlockCommited(BlockHeight, CelestiaHeight),
		RequestBlock(BlockHeight),
	}

	#[derive(Clone)]
	struct StoreMockState<B: Clone, C> {
		height: u64,
		data: HashMap<u64, B>,
		calls: Vec<C>,
	}

	impl<B: Clone, C> StoreMockState<B, C> {
		fn new() -> Self {
			Self { height: 0, data: Default::default(), calls: Default::default() }
		}

		fn get_height(&self) -> u64 {
			self.height - 1
		}

		fn add(&mut self, value: B) {
			self.data.insert(self.height, value);
			self.height += 1;
		}

		fn get_at_height(&self, height: u64) -> Option<B> {
			self.data.get(&height).cloned()
		}

		fn add_call(&mut self, call: C) {
			self.calls.push(call);
		}

		fn into_calls(self) -> Vec<C> {
			self.calls
		}
	}

	impl<B: Clone, C> FromIterator<B> for StoreMockState<B, C> {
		fn from_iter<T: IntoIterator<Item = B>>(iter: T) -> Self {
			let mut state: StoreMockState<B, C> = StoreMockState::new();
			for item in iter {
				state.add(item);
			}
			state
		}
	}

	#[derive(Clone)]
	struct CelestiaMock(Arc<RwLock<StoreMockState<CelestiaBlobData, CelestiaClientCalls>>>);

	impl CelestiaMock {
		fn new(init: Vec<CelestiaBlobData>) -> Self {
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
		) -> Result<Option<CelestiaBlobData>, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::GetBlobsAtHeight(height));
			Ok(state.get_at_height(height.into()))
		}

		async fn send_block(
			&self,
			block: SequencerBlockDigest,
			source: BlockSource,
		) -> Result<(), DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::SendBlock(block.clone(), source));
			state.add(CelestiaBlobData { digests: vec![block] });
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
		async fn notify_block_commited(
			&self,
			block_height: BlockHeight,
			celestia_height: CelestiaHeight,
		) -> Result<(), DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(BlockProviderCalls::NotifyBlockCommited(block_height, celestia_height));
			Ok(())
		}

		async fn request_block(
			&self,
			height: BlockHeight,
		) -> Result<SequencerBlock, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(BlockProviderCalls::RequestBlock(height));
			let block = state.get_at_height(height.into()).unwrap();
			Ok(SequencerBlock { height, block })
		}
	}

	fn test_blocks(count: usize) -> Vec<Block> {
		if count == 0 {
			return vec![];
		}

		let mut rng = rand::thread_rng();
		let mut blocks = Vec::with_capacity(count);
		let genesis = Block::default();
		let mut parent: Id = genesis.id();
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
		let digests = into_digests(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = digests.iter().map(|d| CelestiaBlobData { digests: vec![d.clone()] }).collect();
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
		assert_eq!(block_provider.into_calls().await, vec![]);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_missing_confirmations() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = digests.iter().map(|d| CelestiaBlobData { digests: vec![d.clone()] }).collect();
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
				BlockProviderCalls::NotifyBlockCommited(1.into(), 1.into()),
				BlockProviderCalls::NotifyBlockCommited(2.into(), 2.into()),
			]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_one_behind() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
		let block_provider = BlockProviderMock::new(blocks);
		let blobs = digests
			.iter()
			.take(digests.len() - 1)
			.map(|d| CelestiaBlobData { digests: vec![d.clone()] })
			.collect();
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
				CelestiaClientCalls::SendBlock(
					digests.get(2).cloned().unwrap(),
					BlockSource::Bootstrap
				)
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::NotifyBlockCommited(1.into(), 1.into()),
				BlockProviderCalls::RequestBlock(2.into())
			]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_from_genesis() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
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
				CelestiaClientCalls::SendBlock(
					digests.get(1).cloned().unwrap(),
					BlockSource::Bootstrap
				),
				CelestiaClientCalls::SendBlock(
					digests.get(2).cloned().unwrap(),
					BlockSource::Bootstrap
				)
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::RequestBlock(1.into()),
				BlockProviderCalls::RequestBlock(2.into()),
			]
		);
	}
}
