//! The blob submitter task.

use super::{BlockSource, CelestiaBlobData, CelestiaHeight, ExternalDaNotification};
use crate::block::{BlockHeight, SequencerBlockDigest};

use anyhow::Context;
use celestia_rpc::prelude::*;
use celestia_rpc::{Client, TxConfig};
use celestia_types::nmt::Namespace;
use celestia_types::{AppVersion, Blob};
use tokio::select;
use tokio::sync::mpsc;
use tracing::debug;

use std::mem;
use std::sync::Arc;

// Size limit to ensure inclusion in a Celestia block, as per
// https://docs.celestia.org/how-to-guides/submit-data#maximum-blob-size
const MAX_CELESTIA_BLOB_SIZE: usize = 512 * 1024;

pub(crate) struct BlobSubmitter {
	// The Celestia RPC client
	celestia_client: Arc<Client>,
	// The Celestia namespace
	celestia_namespace: Namespace,
	// Channel to receive digests from foreground
	digest_receiver: mpsc::Receiver<(SequencerBlockDigest, BlockSource)>,
	// Channel to send notifications from Celestia layer
	notifier: mpsc::Sender<ExternalDaNotification>,
}

impl BlobSubmitter {
	pub(crate) fn new(
		celestia_client: Arc<Client>,
		celestia_namespace: Namespace,
		digest_receiver: mpsc::Receiver<(SequencerBlockDigest, BlockSource)>,
		notifier: mpsc::Sender<ExternalDaNotification>,
	) -> Self {
		BlobSubmitter { celestia_client, celestia_namespace, digest_receiver, notifier }
	}

	pub(crate) async fn run(mut self) -> Result<(), anyhow::Error> {
		// Digests accumulated while waiting for client to submit
		let mut buffered_digests: Vec<SequencerBlockDigest> = vec![];
		// Digests accumulated on bootstrap
		let mut bootstrap_digests: Vec<SequencerBlockDigest> = vec![];
		// Size of the accumulated blob data
		let mut total_data_size = 0;
		let mut submit_request = None;
		loop {
			match &mut submit_request {
				None => {
					// No request is currently pending.
					// Grab the accumulated digests, if there are any, and submit them in a blob.
					// Bootstrap digests should be sent ahead of the digests that arrived with
					// submit requests.
					if !buffered_digests.is_empty() || !bootstrap_digests.is_empty() {
						let mut digests = mem::replace(&mut bootstrap_digests, vec![]);
						digests.append(&mut buffered_digests);
						total_data_size = 0;
						submit_request = Some(Box::pin(submit_blob(
							&self.celestia_client,
							self.celestia_namespace.clone(),
							digests,
						)));
					}
				}
				Some(pending_request) => {
					// While a submit request is pending, accumulate blobs.
					// Provide back-pressure if the data size is pushing
					// against the Celestia sanity limit.
					select! {
						res = pending_request => {
							let (block_height, celestia_height) = res?;
							if self
								.notifier
								.send(
									ExternalDaNotification::BlockCommitted(block_height, celestia_height)
								)
								.await
								.is_err() {
								debug!("failed to send notification, shutting down");
								break;
							}
							submit_request = None;
						}
						next = self.digest_receiver.recv(), if total_data_size + SequencerBlockDigest::DIGEST_SIZE <= MAX_CELESTIA_BLOB_SIZE => {
							match next {
								None => break,
								Some((digest, BlockSource::Input)) => {
									total_data_size += digest.id.as_bytes().len();
									buffered_digests.push(digest);
								}
								Some((digest, BlockSource::Bootstrap)) => {
									total_data_size += digest.id.as_bytes().len();
									bootstrap_digests.push(digest);
								}
							}
						}
					}
				}
			}
		}
		Ok(())
	}
}

async fn submit_blob(
	celestia_client: &Client,
	namespace: Namespace,
	digests: Vec<SequencerBlockDigest>,
) -> Result<(BlockHeight, CelestiaHeight), anyhow::Error> {
	let last_block_height =
		digests.last().map(|digest| digest.height).expect("array of digests is empty");
	let data = CelestiaBlobData { digests };
	let serialized_data = bcs::to_bytes(&data)?;
	let blob = Blob::new(namespace, serialized_data, AppVersion::V2)?;
	let config = TxConfig::default();
	// config.with_gas(2);
	let celestia_height = celestia_client
		.blob_submit(&[blob], config)
		.await
		.context("failed to submit the blob")?;
	Ok((last_block_height, CelestiaHeight::new(celestia_height)))
}
