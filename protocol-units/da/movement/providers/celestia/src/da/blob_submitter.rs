//! The blob submitter task.

use std::mem;
use std::sync::Arc;

use anyhow::Context;
use celestia_rpc::prelude::*;
use celestia_rpc::{Client, TxConfig};
use celestia_types::Blob;
use tokio::select;
use tokio::sync::mpsc;

// Size limit to ensure inclusion in a Celestia block, as per
// https://docs.celestia.org/how-to-guides/submit-data#maximum-blob-size
// Note that this is currently not the hard limit: for simplicity,
// the next blob is still added to the buffer. Subtly relying on the gRPC
// message size limit to not end up with an oversized blob request.
const CELESTIA_BLOB_SIZE_THRESHOLD: usize = 512 * 1024;

pub(crate) struct BlobSubmitter {
	// The Celestia RPC client
	celestia_client: Arc<Client>,
	// Channel to receive blobs from foreground
	blob_receiver: mpsc::Receiver<Blob>,
}

impl BlobSubmitter {
	pub(crate) fn new(celestia_client: Arc<Client>, blob_receiver: mpsc::Receiver<Blob>) -> Self {
		BlobSubmitter { celestia_client, blob_receiver }
	}

	pub(crate) async fn run(mut self) -> Result<(), anyhow::Error> {
		// Blobs accumulated while waiting for client to submit
		let mut buffered_blobs = vec![];
		// Size of the accumulated blob data
		let mut total_data_size = 0;
		let mut submit_request = None;
		loop {
			match &mut submit_request {
				None => {
					// No request is currently pending.
					// Grab the accumulated blobs, if there are any, and submit them.
					if !buffered_blobs.is_empty() {
						let blobs = mem::replace(&mut buffered_blobs, vec![]);
						submit_request = Some(Box::pin(submit_blobs(&self.celestia_client, blobs)));
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
						next = self.blob_receiver.recv(), if total_data_size < CELESTIA_BLOB_SIZE_THRESHOLD => {
							match next {
								None => break,
								Some(blob) => {
									total_data_size += blob.data.len();
									buffered_blobs.push(blob);
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

async fn submit_blobs(celestia_client: &Client, blobs: Vec<Blob>) -> Result<u64, anyhow::Error> {
	let config = TxConfig::default();
	// config.with_gas(2);
	celestia_client
		.blob_submit(&blobs, config)
		.await
		.context("failed to submit the blob")
}
