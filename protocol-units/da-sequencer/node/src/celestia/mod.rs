use crate::block::BlockHeight;
use crate::block::SequencerBlockDigest;
use crate::celestia::blob::Blob;
use crate::error::DaSequencerError;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::Sender;
use url::Url;

mod blob;

/// Functions to implement to save block digest in an external DA like Celestia
pub trait DaSequencerExternDaClient {
	/// Create the Celestia client and all async process to manage celestia access.
	fn new(
		connection_url: Url,
		notifier: Sender<CelestiaNotifier>,
	) -> Pin<Box<dyn Future<Output = Self> + Send>>;

	/// send the given block to Celestia.
	/// The block is not immediatly sent but aggergated in a blob
	/// Until the client can send it to celestia.
	async fn send_block(
		&self,
		block: &SequencerBlockDigest,
	) -> Pin<Box<dyn Future<Output = std::result::Result<(), DaSequencerError>> + Send>>;

	/// Get the blob from celestia at the given height.
	async fn get_blob_at_height(&self) -> Pin<Box<dyn Future<Output = Blob> + Send>>;

	/// Bootstrap the Celestia client to recovert from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recovert this missing block to all block of the network are sent to celestia.
	/// The basic algorythm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// The missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	async fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_sent_block_height: BlockHeight,
		last_notified_celestia_height: CelestiaHeight,
	) -> Pin<Box<dyn Future<Output = std::result::Result<(), DaSequencerError>> + Send>>;
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

/// Message, use to notify CelestiaClient activities.
#[derive(Clone, Debug)]
pub enum CelestiaNotifier {
	/// Notify that the block at specifed height has been sent to the Celestia network.
	BlockSent(BlockHeight),
	/// Notify that the block at specified height has been commited on celestia network
	BlockCommited(BlockHeight, CelestiaHeight),
	/// Ask to send the block at specified height to the Celestia client.
	/// Use during bootstrap to request block that are missing on Celestia network.
	RequestBlockAtHeight(BlockHeight),
}

pub struct CelestiaClient {
	notifier: Sender<CelestiaNotifier>,
}

impl CelestiaClient {
	/// Create the Celestia client and all async process to manage celestia access.
	pub async fn new(connection_url: Url, notifier: Sender<CelestiaNotifier>) -> Self {
		CelestiaClient { notifier }
	}

	/// send the given block to Celestia.
	/// The block is not immediatly sent but aggergated in a blob
	/// Until the client can send it to celestia.
	pub async fn send_block(
		&self,
		block: &SequencerBlockDigest,
	) -> std::result::Result<(), DaSequencerError> {
		todo!()
	}

	/// Get the blob from celestia at the given height.
	pub async fn get_blob_at_height(&self) -> Blob {
		todo!()
	}

	/// Bootstrap the Celestia client to recovert from missing block.
	/// In case of crash for example, block sent to Celestia can be behind the block created by the network.
	/// The role of this function is to recovert this missing block to all block of the network are sent to celestia.
	/// The basic algorythm is start from 'last_sent_block_height' to 'current_block_height' and request using the notifier channel
	/// The missing block. Not sure last_notified_celestia_height is useful.
	/// During this boostrap new block are sent to the client.
	/// These block should be buffered until the boostrap is done then sent after in order.
	pub async fn bootstrap(
		&self,
		current_block_height: BlockHeight,
		last_sent_block_height: BlockHeight,
		last_notified_celestia_height: CelestiaHeight,
	) -> std::result::Result<(), DaSequencerError> {
		todo!()
	}
}
