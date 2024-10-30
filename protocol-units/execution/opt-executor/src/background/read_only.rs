use super::Error;

use aptos_mempool::MempoolClientRequest;

use futures::channel::mpsc::Receiver;
use futures::StreamExt;
use tracing::debug;

pub struct NullMempool {
	// The receiver for the mempool client.
	mempool_client_receiver: Receiver<MempoolClientRequest>,
}

impl NullMempool {
	pub fn new(mempool_client_receiver: Receiver<MempoolClientRequest>) -> Self {
		Self { mempool_client_receiver }
	}

	pub async fn run(mut self) -> Result<(), Error> {
		while let Some(request) = self.mempool_client_receiver.next().await {
			match request {
				MempoolClientRequest::SubmitTransaction(_, _) => {
					panic!("SubmitTransaction received in read-only mode");
				}
				MempoolClientRequest::GetTransactionByHash(_hash, sender) => {
					sender.send(None).unwrap_or_else(|_| {
						debug!("GetTransactionByHash request canceled");
					});
				}
			}
		}
		Ok(())
	}
}
