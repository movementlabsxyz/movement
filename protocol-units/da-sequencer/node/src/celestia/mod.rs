use crate::block::SequencerBlockDigest;
use crate::block::{BlockHeight, SequencerBlock};
use crate::celestia::blob::Blob;
use crate::error::DaSequencerError;
use std::future::Future;
use std::ops::Add;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

mod blob;

/// Functions to implement to save block digest in an external DA like Celestia
pub trait ExternalDa {
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
		last_notified_celestia_height: CelestiaHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send;
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

impl<T: Into<u64>> Add<T> for CelestiaHeight {
	type Output = Self;

	fn add(self, rhs: T) -> Self::Output {
		CelestiaHeight(self.0 + rhs.into())
	}
}

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
	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<Blob>, DaSequencerError>> + Send;
}

const DELAY_SECONDS_BEFORE_BOOTSTRAPPING: Duration = Duration::from_secs(12);

pub struct CelestiaExternalDa<C: CelestiaClient> {
	notifier: mpsc::Sender<ExternalDaNotification>,
	celestia_client: C,
}

impl<C: CelestiaClient> CelestiaExternalDa<C> {
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

impl<C: CelestiaClient + Sync> ExternalDa for CelestiaExternalDa<C> {
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
	async fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_finalized_celestia_height: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		// wait to ensure that no blob is pending in the Celestia network
		tokio::time::sleep(DELAY_SECONDS_BEFORE_BOOTSTRAPPING).await;

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
		let mut block = self.request_block(BlockAt::Digest(digest.clone())).await?;

		// Step 3: Request and send all missing blocks
		for height in (block.height.0 + 1)..=current_block_height.0 {
			block = self.request_block(BlockAt::Height(BlockHeight(height))).await?;
			self.send_block(block.get_block_digest()).await?;
		}

		Ok(())
	}
}
