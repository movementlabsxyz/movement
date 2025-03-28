//! The blob submitter task.

use super::{CelestiaBlobData, ExternalDaNotification};
use crate::block::SequencerBlockDigest;

use anyhow::Context;
use celestia_rpc::prelude::*;
use celestia_rpc::{Client, TxConfig};
use celestia_types::nmt::Namespace;
use celestia_types::{AppVersion, Blob};
use tokio::select;
use tokio::sync::mpsc;

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
	digest_receiver: mpsc::Receiver<SequencerBlockDigest>,
	// Channel to send notifications from Celestia layer
	notifier: mpsc::Sender<ExternalDaNotification>,
}

impl BlobSubmitter {
	pub(crate) fn new(
		celestia_client: Arc<Client>,
		celestia_namespace: Namespace,
		digest_receiver: mpsc::Receiver<SequencerBlockDigest>,
		notifier: mpsc::Sender<ExternalDaNotification>,
	) -> Self {
		BlobSubmitter { celestia_client, celestia_namespace, digest_receiver, notifier }
	}

	pub(crate) async fn run(mut self) -> Result<(), anyhow::Error> {
		// Digests accumulated while waiting for client to submit
		let mut buffered_digests = vec![];
		// Size of the accumulated blob data
		let mut total_data_size = 0;
		let mut submit_request = None;
		loop {
			match &mut submit_request {
				None => {
					// No request is currently pending.
					// Grab the accumulated digests, if there are any, and submit them in a blob.
					if !buffered_digests.is_empty() {
						let digests = mem::replace(&mut buffered_digests, vec![]);
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
							res?;
							submit_request = None;
						}
						next = self.digest_receiver.recv(), if total_data_size + SequencerBlockDigest::DIGEST_SIZE <= MAX_CELESTIA_BLOB_SIZE => {
							match next {
								None => break,
								Some(digest) => {
									total_data_size += digest.id.len();
									buffered_digests.push(digest);
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
) -> Result<u64, anyhow::Error> {
	let data = CelestiaBlobData { digests };
	let serialized_data = bcs::to_bytes(&data)?;
	let blob = Blob::new(namespace, serialized_data, AppVersion::V2)?;
	let config = TxConfig::default();
	// config.with_gas(2);
	celestia_client
		.blob_submit(&[blob], config)
		.await
		.context("failed to submit the blob")
}
