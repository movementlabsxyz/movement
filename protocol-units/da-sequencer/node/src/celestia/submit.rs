//! The blob submitter task.

use super::{BlockSource, CelestiaBlobData, CelestiaHeight, ExternalDaNotification};
use crate::block::BlockHeight;

use anyhow::Context;
use celestia_rpc::prelude::*;
use celestia_rpc::{Client, TxConfig};
use celestia_types::nmt::Namespace;
use celestia_types::{AppVersion, Blob};
use tokio::select;
use tokio::sync::mpsc;
use tracing::debug;

use movement_types::block;
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
	id_receiver: mpsc::Receiver<(block::Id, BlockSource)>,
	// Channel to send notifications from Celestia layer
	notifier: mpsc::Sender<ExternalDaNotification>,
}

impl BlobSubmitter {
	pub(crate) fn new(
		celestia_client: Arc<Client>,
		celestia_namespace: Namespace,
		id_receiver: mpsc::Receiver<(block::Id, BlockSource)>,
		notifier: mpsc::Sender<ExternalDaNotification>,
	) -> Self {
		BlobSubmitter { celestia_client, celestia_namespace, id_receiver, notifier }
	}

	pub(crate) async fn run(mut self) -> Result<(), anyhow::Error> {
		// Digests accumulated while waiting for client to submit
		let mut buffered_ids: Vec<block::Id> = vec![];
		// Digests accumulated on bootstrap
		let mut bootstrap_ids: Vec<block::Id> = vec![];
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
					if !buffered_ids.is_empty() || !bootstrap_ids.is_empty() {
						let mut digests = mem::replace(&mut bootstrap_ids, vec![]);
						digests.append(&mut buffered_ids);
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
							let (block_ids, celestia_height) = res?;
							if self
								.notifier
								.send(
									ExternalDaNotification::BlocksCommitted { block_ids, celestia_height }
								)
								.await
								.is_err() {
								debug!("failed to send notification, shutting down");
								break;
							}
							submit_request = None;
						}
						next = self.id_receiver.recv(), if total_data_size + block::ID_SIZE <= MAX_CELESTIA_BLOB_SIZE => {
							match next {
								None => break,
								Some((id, BlockSource::Input)) => {
									total_data_size += block::ID_SIZE;
									buffered_ids.push(id);
								}
								Some((id, BlockSource::Bootstrap)) => {
									total_data_size += block::ID_SIZE;
									bootstrap_ids.push(id);
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
	ids: Vec<block::Id>,
) -> Result<(Vec<block::Id>, CelestiaHeight), anyhow::Error> {
	let data = CelestiaBlobData::from(ids.clone());
	let serialized_data = bcs::to_bytes(&data)?;
	let blob = Blob::new(namespace, serialized_data, AppVersion::V2)?;
	let config = TxConfig::default();
	// config.with_gas(2);
	let celestia_height = celestia_client
		.blob_submit(&[blob], config)
		.await
		.context("failed to submit the blob")?;
	Ok((ids, CelestiaHeight::from(celestia_height)))
}
