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

	fn get_blobs_at_height(
		&self,
		height: u64,
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
	async fn send_block(&self, _block: SequencerBlockDigest) -> Result<(), DaSequencerError> {
		todo!()
	}

	/// Get the blob from celestia at the given height.
	async fn get_blobs_at_height(
		&self,
		_height: CelestiaHeight,
	) -> Result<Option<Vec<Blob>>, DaSequencerError> {
		todo!()
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
		let mut block = self.block_provider.request_block(BlockAt::Digest(digest.clone())).await?;

		// Step 3: Request and send all missing blocks
		for height in (block.height.0 + 1)..=current_block_height.0 {
			block = self.block_provider.request_block(BlockAt::Height(BlockHeight(height))).await?;
			self.send_block(block.get_block_digest()).await?;
		}

		Ok(())
	}
}
