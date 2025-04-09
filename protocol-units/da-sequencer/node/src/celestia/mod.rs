pub mod blob;
mod client;
mod submit;

use crate::block::SequencerBlockDigest;
use crate::block::{BlockHeight, SequencerBlock};
use crate::celestia::blob::CelestiaBlobData;
use crate::error::DaSequencerError;
use tokio::sync::{mpsc, oneshot};

use std::future::Future;
use std::time::Duration;

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
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

impl CelestiaHeight {
	pub fn new(raw: u64) -> Self {
		CelestiaHeight(raw)
	}
}

/// Message, use to notify CelestiaClient activities.
#[derive(Debug)]
pub enum ExternalDaNotification {
	/// Notify that the block at specified height has been commited on celestia network
	BlockCommitted(BlockHeight, CelestiaHeight),
	/// Ask to send the block at specified height to the Celestia client.
	/// Use during bootstrap to request block that are missing on Celestia network.
	RequestBlock { at: BlockAt, callback: oneshot::Sender<Option<SequencerBlock>> },
}

/// Source for the block digest
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BlockSource {
	/// The block has arrived on the DA service.
	Input,
	/// The block has been recovered in bootstrap.
	Bootstrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockAt {
	Height(BlockHeight),
	Digest([u8; SequencerBlockDigest::DIGEST_SIZE]),
}

pub trait CelestiaClientOps {
	fn get_current_height(
		&self,
	) -> impl Future<Output = Result<CelestiaHeight, DaSequencerError>> + Send;

	fn get_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Option<CelestiaBlobData>, DaSequencerError>> + Send;

	fn send_block(
		&self,
		block: SequencerBlockDigest,
		source: BlockSource,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

const DELAY_SECONDS_BEFORE_BOOTSTRAPPING: Duration = Duration::from_secs(12);

#[derive(Clone)]
pub struct CelestiaExternalDa<C: CelestiaClientOps + Clone> {
	notifier: mpsc::Sender<ExternalDaNotification>,
	celestia_client: C,
}

impl<C: CelestiaClientOps + Sync + Clone> CelestiaExternalDa<C> {
	/// Create the Celestia client and all async process to manage celestia access.
	pub async fn new(celestia_client: C, notifier: mpsc::Sender<ExternalDaNotification>) -> Self {
		CelestiaExternalDa { notifier, celestia_client }
	}

	async fn request_block(&self, at: BlockAt) -> Result<SequencerBlock, DaSequencerError> {
		let (tx, rx) = oneshot::channel();
		let request = ExternalDaNotification::RequestBlock { at, callback: tx };
		self.notifier.send(request).await.map_err(|e| {
			DaSequencerError::BlockRetrieval(format!("Broken notifier channel: {}", e))
		})?;
		let block = rx.await.map_err(|e| {
			DaSequencerError::BlockRetrieval(format!("Broken notifier channel: {}", e))
		})?;
		let block = block
			.ok_or(DaSequencerError::BlockRetrieval(format!("Block at {:?} not found", at)))?;
		Ok(block)
	}
}

impl<C: CelestiaClientOps + Sync + Clone> DaSequencerExternalDa for CelestiaExternalDa<C> {
	/// Send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// until the client can send it to celestia.
	async fn send_block(&self, block: SequencerBlockDigest) -> Result<(), DaSequencerError> {
		self.celestia_client.send_block(block, BlockSource::Input).await
	}

	/// Get the blob from celestia at the given height.
	async fn get_blobs_at_height(
		&self,
		_height: CelestiaHeight,
	) -> Result<Option<CelestiaBlobData>, DaSequencerError> {
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
			.and_then(|mut blob_data| blob_data.digests.pop())
			.ok_or(DaSequencerError::ExternalDaBootstrap(format!(
				"Celestia returned no blobs or an empty last blob at height {}",
				last_finalized_celestia_height.0
			)))?;

		// Step 2: Get the Block for digest
		let mut block = self.request_block(BlockAt::Digest(*digest.id.as_bytes())).await?;

		// Step 3: Request and send all missing blocks
		for height in (block.height.0 + 1)..=current_block_height.0 {
			block = self.request_block(BlockAt::Height(BlockHeight(height))).await?;
			self.celestia_client
				.send_block(block.get_block_digest(), BlockSource::Bootstrap)
				.await?;
		}

		Ok(())
	}
}
