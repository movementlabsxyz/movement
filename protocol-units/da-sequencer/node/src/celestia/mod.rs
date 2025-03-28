use crate::block::SequencerBlockDigest;
use crate::block::{BlockHeight, SequencerBlock};
use crate::celestia::blob::Blob;
use crate::error::DaSequencerError;
use std::future::Future;
use std::ops::Add;
use std::pin::Pin;
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
	) -> Pin<Box<dyn Future<Output = Result<(), DaSequencerError>> + Send + '_>>;

	/// Get the blob from celestia at the given height.
	fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> Pin<Box<dyn Future<Output = Result<Option<Vec<Blob>>, DaSequencerError>> + Send + '_>>;

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
		last_sent_block_height: BlockHeight,
		last_notified_celestia_height: CelestiaHeight,
	) -> Pin<Box<dyn Future<Output = Result<(), DaSequencerError>> + Send + '_>>;
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

impl<T: Into<u64>> Add<T> for CelestiaHeight {
	type Output = Self;

	fn add(self, rhs: T) -> Self::Output {
		CelestiaHeight(self.0.saturating_add(rhs.into()))
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
	RequestBlockAtHeight { height: BlockHeight, callback: oneshot::Sender<SequencerBlock> },
}

pub trait CelestiaClient {
	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<Blob>, DaSequencerError>> + Send + '_>>;
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
}

impl<C: CelestiaClient + Sync> ExternalDa for CelestiaExternalDa<C> {
	/// Send the given block to Celestia.
	/// The block is not immediately sent but aggregated in a blob
	/// until the client can send it to celestia.
	fn send_block(
		&self,
		_block: SequencerBlockDigest,
	) -> Pin<Box<dyn Future<Output = Result<(), DaSequencerError>> + Send + '_>> {
		todo!()
	}

	/// Get the blob from celestia at the given height.
	fn get_blobs_at_height(
		&self,
		_height: CelestiaHeight,
	) -> Pin<Box<dyn Future<Output = Result<Option<Vec<Blob>>, DaSequencerError>> + Send + '_>> {
		todo!()
	}

	/// Bootstrap the Celestia client to recover from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recover this missing block to all block of the network are sent to celestia.
	/// The basic algorithm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// the missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		_last_sent_block_height: BlockHeight,
		last_finalized_celestia_height: CelestiaHeight,
	) -> Pin<Box<dyn Future<Output = Result<(), DaSequencerError>> + Send + '_>> {
		Box::pin(async move {
			// wait to ensure that no blob is pending in the Celestia network
			tokio::time::sleep(DELAY_SECONDS_BEFORE_BOOTSTRAPPING).await;

			// Step 1: Get last digest in the last finalized blob
			let blobs = self.get_blobs_at_height(last_finalized_celestia_height).await?;
			// TODO: Handle all of this gracefully with a DaSequencerError
			let blobs = blobs.expect("get_blobs_at_height returned None");
			let last = blobs.last().expect("get_blobs_at_height returned no Blobs");
			let _digest = last.0.last().expect("get_blobs_at_height returned an empty Blob");

			// Step 2: Get the Block for digest
			// TODO: Request the block for digest
			let block: SequencerBlock = SequencerBlock::default();

			// Step 3: Request and send all missing blocks
			for height in block.get_height().0..=current_block_height.0 {
				let (tx, rx) = oneshot::channel();
				let request = ExternalDaNotification::RequestBlockAtHeight {
					height: BlockHeight(height),
					callback: tx,
				};
				self.notifier.send(request).await.unwrap(); // TODO: Handle send error
				let block = rx.await.unwrap(); // TODO: Handle recv error
				self.send_block(block.get_block_digest()).await?;
			}

			Ok(())
		})
	}
}
