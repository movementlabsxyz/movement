use crate::block::SequencerBlockDigest;
use crate::block::{BlockHeight, SequencerBlock};
use crate::celestia::blob::Blob;
use crate::error::DaSequencerError;
use std::future::Future;
use std::ops::Add;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub mod blob;

/// Functions to implement to save block digest in an external DA like Celestia
pub trait DaSequencerExternalDa: Clone {
	/// send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// Until the client can send it to celestia.
	fn send_block(
		&self,
		block: SequencerBlockDigest,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	/// Get the blob from celestia at the given height.
	fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<Vec<Blob>>, DaSequencerError>> + Send;

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
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

/// Message, use to notify CelestiaClient activities.
#[derive(Debug)]
pub enum ExternalDaNotification {
	/// Notify that the block at specified height has been sent to the Celestia network.
	BlockSent(BlockHeight),
	/// Notify that the block at specified height has been commited on celestia network
	BlockCommited(BlockHeight, CelestiaHeight),
	/// Ask to send the block at specified height to the Celestia client.
	/// Use during bootstrap to request block that are missing on Celestia network.
	RequestBlock { at: BlockAt, callback: oneshot::Sender<Option<SequencerBlock>> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockAt {
	Height(BlockHeight),
	Digest(SequencerBlockDigest),
}

pub trait CelestiaClient {
	fn get_current_height(
		&self,
	) -> impl Future<Output = Result<CelestiaHeight, DaSequencerError>> + Send;

	fn send_blob(&self, blob: Blob) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<Vec<Blob>>, DaSequencerError>> + Send;
}

pub trait BlockProvider {
	fn notify_block_sent(
		&self,
		block_height: BlockHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	fn notify_block_commited(
		&self,
		block_height: BlockHeight,
		celestia_height: CelestiaHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;

	fn request_block(
		&self,
		at: BlockAt,
	) -> impl Future<Output = Result<SequencerBlock, DaSequencerError>> + Send;
}

#[derive(Clone)]
pub struct ChannelBlockProvider {
	notifier: mpsc::Sender<ExternalDaNotification>,
}

impl ChannelBlockProvider {
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

impl BlockProvider for ChannelBlockProvider {
	async fn notify_block_sent(&self, block_height: BlockHeight) -> Result<(), DaSequencerError> {
		self.notify(ExternalDaNotification::BlockSent(block_height)).await
	}

	async fn notify_block_commited(
		&self,
		block_height: BlockHeight,
		celestia_height: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		self.notify(ExternalDaNotification::BlockCommited(block_height, celestia_height))
			.await
	}

	async fn request_block(&self, at: BlockAt) -> Result<SequencerBlock, DaSequencerError> {
		let (tx, rx) = oneshot::channel();
		self.notify(ExternalDaNotification::RequestBlock { at, callback: tx }).await?;
		let block = rx.await.map_err(|e| {
			DaSequencerError::ChannelError(format!("Broken notifier channel: {}", e))
		})?;
		let block = block
			.ok_or(DaSequencerError::BlockRetrieval(format!("Block at {:?} not found", at)))?;
		Ok(block)
	}
}

const DELAY_SECONDS_BEFORE_BOOTSTRAPPING: Duration = Duration::from_secs(12);

#[derive(Clone)]
pub struct CelestiaExternalDa<B: BlockProvider + Sync + Clone, C: CelestiaClient + Sync + Clone> {
	block_provider: B,
	celestia_client: C,
}

impl<B: BlockProvider + Sync + Clone, C: CelestiaClient + Sync + Clone> CelestiaExternalDa<B, C> {
	/// Create the Celestia client and all async process to manage celestia access.
	pub fn new(block_provider: B, celestia_client: C) -> Self {
		CelestiaExternalDa { block_provider, celestia_client }
	}
}

impl<C: CelestiaClient + Sync + Clone> CelestiaExternalDa<ChannelBlockProvider, C> {
	pub fn with_notifier(
		notifier: mpsc::Sender<ExternalDaNotification>,
		celestia_client: C,
	) -> Self {
		Self::new(ChannelBlockProvider::new(notifier), celestia_client)
	}
}

impl<B: BlockProvider + Sync + Clone, C: CelestiaClient + Sync + Clone> DaSequencerExternalDa
	for CelestiaExternalDa<B, C>
{
	/// Send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// until the client can send it to celestia.
	async fn send_block(&self, block: SequencerBlockDigest) -> Result<(), DaSequencerError> {
		self.celestia_client.send_blob(Blob(vec![block])).await
	}

	/// Get the blob from celestia at the given height.
	async fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> Result<Option<Vec<Blob>>, DaSequencerError> {
		self.celestia_client.get_blobs_at_height(height).await
	}

	/// Bootstrap the Celestia client to recover from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recover this missing block to all block of the network are sent to celestia.
	/// The basic algorithm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// the missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	async fn bootstrap(&self, current_block_height: BlockHeight) -> Result<(), DaSequencerError> {
		// wait to ensure that no blob is pending in the Celestia network
		tokio::time::sleep(DELAY_SECONDS_BEFORE_BOOTSTRAPPING).await;

		let last_finalized_celestia_height = self.celestia_client.get_current_height().await?;
		let height_from = if last_finalized_celestia_height.0 == 0 {
			// Step 1: No blobs have been sent to Celestia yet
			1
		} else {
			// Step 1: Get last digest in the last finalized blob
			let digest = self
				.get_blobs_at_height(last_finalized_celestia_height)
				.await?
				.and_then(|mut blobs| blobs.pop())
				.and_then(|mut digest| digest.0.pop())
				.ok_or(DaSequencerError::ExternalDaBootstrap(format!(
					"Celestia returned no blobs or an empty last blob at height {}",
					last_finalized_celestia_height.0
				)))?;

			// Step 2: Get the Block for digest
			let mut block =
				self.block_provider.request_block(BlockAt::Digest(digest.clone())).await?;
			block.height.0 + 1
		};

		// Step 3: Request and send all missing blocks
		for height in height_from..=current_block_height.0 {
			let block =
				self.block_provider.request_block(BlockAt::Height(BlockHeight(height))).await?;
			self.send_block(block.get_block_digest()).await?;
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

	fn digest(block: &Block) -> SequencerBlockDigest {
		SequencerBlockDigest(*block.id().as_bytes())
	}

	fn into_digests(blocks: &[Block]) -> Vec<SequencerBlockDigest> {
		blocks.iter().map(digest).collect()
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum CelestiaClientCalls {
		#[default]
		Noop,
		GetCurrentHeight,
		SendBlob(Blob),
		GetBlobsAtHeight(CelestiaHeight),
	}

	#[derive(Clone, Default, Debug, Eq, PartialEq)]
	enum BlockProviderCalls {
		#[default]
		Noop,
		RequestBlock(BlockAt),
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
			self.height
		}

		fn add(&mut self, value: B) {
			self.height += 1;
			self.data.insert(self.height, value);
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

	#[derive(Clone)]
	struct CelestiaMock(Arc<RwLock<StoreMockState<Blob, CelestiaClientCalls>>>);

	impl CelestiaMock {
		fn new(init: Vec<Blob>) -> Self {
			let mut state = StoreMockState::new();
			for blob in init.into_iter() {
				state.add(blob);
			}
			Self(Arc::new(RwLock::new(state)))
		}

		async fn into_calls(self) -> Vec<CelestiaClientCalls> {
			let mut state = self.0.write().await;
			let state = std::mem::replace(&mut *state, StoreMockState::new());
			state.into_calls()
		}
	}

	impl CelestiaClient for CelestiaMock {
		async fn get_current_height(&self) -> Result<CelestiaHeight, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::GetCurrentHeight);
			Ok(CelestiaHeight(state.get_height()))
		}

		async fn send_blob(&self, blob: Blob) -> Result<(), DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::SendBlob(blob.clone()));
			state.add(blob);
			Ok(())
		}

		async fn get_blobs_at_height(
			&self,
			height: CelestiaHeight,
		) -> Result<Option<Vec<Blob>>, DaSequencerError> {
			let mut state = self.0.write().await;
			state.add_call(CelestiaClientCalls::GetBlobsAtHeight(height));
			let blobs = state.get_at_height(height.0).map(|blob| vec![blob]);
			Ok(blobs)
		}
	}

	#[derive(Clone)]
	struct BlockProviderMockState {
		store: StoreMockState<Block, BlockProviderCalls>,
		index: HashMap<SequencerBlockDigest, BlockHeight>,
	}

	impl Default for BlockProviderMockState {
		fn default() -> Self {
			BlockProviderMockState { store: StoreMockState::new(), index: Default::default() }
		}
	}

	impl BlockProviderMockState {
		fn new() -> Self {
			Default::default()
		}
	}

	#[derive(Default, Clone)]
	struct BlockProviderMock(Arc<RwLock<BlockProviderMockState>>);

	impl BlockProviderMock {
		fn new(init: Vec<Block>) -> Self {
			let mut state = BlockProviderMockState::new();
			for block in init.into_iter() {
				let digest = digest(&block);
				state.store.add(block);
				state.index.insert(digest, BlockHeight(state.store.get_height()));
			}
			Self(Arc::new(RwLock::new(state)))
		}

		async fn get_height(&self) -> BlockHeight {
			BlockHeight(self.0.read().await.store.get_height())
		}

		async fn into_calls(self) -> Vec<BlockProviderCalls> {
			let mut state = self.0.write().await;
			let state = std::mem::take(&mut *state);
			state.store.into_calls()
		}
	}

	impl BlockProvider for BlockProviderMock {
		async fn notify_block_sent(
			&self,
			block_height: BlockHeight,
		) -> Result<(), DaSequencerError> {
			unimplemented!()
		}

		async fn notify_block_commited(
			&self,
			block_height: BlockHeight,
			celestia_height: CelestiaHeight,
		) -> Result<(), DaSequencerError> {
			unimplemented!()
		}

		async fn request_block(&self, at: BlockAt) -> Result<SequencerBlock, DaSequencerError> {
			let mut state = self.0.write().await;
			state.store.add_call(BlockProviderCalls::RequestBlock(at));
			let height = match at {
				BlockAt::Height(h) => h.0,
				BlockAt::Digest(d) => state.index.get(&d).unwrap().0,
			};
			let block = state.store.get_at_height(height).unwrap();
			Ok(SequencerBlock { height: BlockHeight(height), block })
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
		let celestia = CelestiaMock::new(vec![]);
		let block_provider = BlockProviderMock::new(vec![]);
		let current_height = block_provider.get_height().await;
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());
		assert!(da.bootstrap(current_height).await.is_ok());
		assert_eq!(celestia.into_calls().await, vec![CelestiaClientCalls::GetCurrentHeight]);
		assert_eq!(block_provider.into_calls().await, vec![]);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_in_sync() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
		let celestia = CelestiaMock::new(vec![Blob(digests.clone())]);
		let block_provider = BlockProviderMock::new(blocks);
		let current_height = block_provider.get_height().await;
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());
		assert!(da.bootstrap(current_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetCurrentHeight,
				CelestiaClientCalls::GetBlobsAtHeight(CelestiaHeight(1))
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![BlockProviderCalls::RequestBlock(BlockAt::Digest(
				digests.last().cloned().unwrap()
			))]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_behind() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
		let blobs = digests.iter().take(digests.len() - 1).map(|d| Blob(vec![d.clone()])).collect();
		let celestia = CelestiaMock::new(blobs);
		let block_provider = BlockProviderMock::new(blocks);
		let current_height = block_provider.get_height().await;
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());
		assert!(da.bootstrap(current_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetCurrentHeight,
				CelestiaClientCalls::GetBlobsAtHeight(CelestiaHeight(2)),
				CelestiaClientCalls::SendBlob(Blob(vec![digests.last().cloned().unwrap()]))
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::RequestBlock(BlockAt::Digest(
					digests.get(digests.len() - 2).cloned().unwrap()
				)),
				BlockProviderCalls::RequestBlock(BlockAt::Height(BlockHeight(3)))
			]
		);
	}

	#[tokio::test]
	async fn test_celestia_external_da_bootstrap_from_genesis() {
		let blocks = test_blocks(3);
		let digests = into_digests(blocks.as_slice());
		let celestia = CelestiaMock::new(vec![]);
		let block_provider = BlockProviderMock::new(blocks);
		let current_height = block_provider.get_height().await;
		let da = CelestiaExternalDa::new(block_provider.clone(), celestia.clone());
		assert!(da.bootstrap(current_height).await.is_ok());
		assert_eq!(
			celestia.into_calls().await,
			vec![
				CelestiaClientCalls::GetCurrentHeight,
				CelestiaClientCalls::SendBlob(Blob(vec![digests.get(0).cloned().unwrap()])),
				CelestiaClientCalls::SendBlob(Blob(vec![digests.get(1).cloned().unwrap()])),
				CelestiaClientCalls::SendBlob(Blob(vec![digests.get(2).cloned().unwrap()])),
			]
		);
		assert_eq!(
			block_provider.into_calls().await,
			vec![
				BlockProviderCalls::RequestBlock(BlockAt::Height(BlockHeight(1))),
				BlockProviderCalls::RequestBlock(BlockAt::Height(BlockHeight(2))),
				BlockProviderCalls::RequestBlock(BlockAt::Height(BlockHeight(3)))
			]
		);
	}
}
