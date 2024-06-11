use tokio_stream::Stream;
use tracing::{debug, info};

use std::{fmt::Debug, path::PathBuf};

use celestia_rpc::HeaderClient;

use m1_da_light_node_grpc::light_node_service_server::LightNodeService;
// FIXME: glob imports are bad style
use m1_da_light_node_grpc::*;
use memseq::{Sequencer, Transaction};

use crate::v1::{passthrough::LightNodeV1 as LightNodeV1PassThrough, LightNodeV1Operations};

#[derive(Clone)]
pub struct LightNodeV1 {
	pub pass_through: LightNodeV1PassThrough,
	pub memseq: memseq::Memseq<memseq::RocksdbMempool>,
}

impl Debug for LightNodeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LightNodeV1")
            .field("pass_through", &self.pass_through)
            .finish()
    }
}

impl LightNodeV1Operations for LightNodeV1 {
	async fn try_from_env_toml_file() -> Result<Self, anyhow::Error> {
		info!("Initializing LightNodeV1 in sequencer mode from environment.");

		let pass_through = LightNodeV1PassThrough::try_from_env_toml_file().await?;
		info!("Initialized pass through for LightNodeV1 in sequencer mode.");

		let memseq_path = pass_through.config.try_memseq_config()?.try_sequencer_database_path()?;

		let memseq = memseq::Memseq::try_move_rocks(
			PathBuf::from(memseq_path),
		)?;
		info!("Initialized Memseq with Move Rocks for LightNodeV1 in sequencer mode.");

		Ok(Self { pass_through, memseq })
	}

	fn try_service_address(&self) -> Result<String, anyhow::Error> {
		self.pass_through.try_service_address()
	}

	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		self.run_block_proposer().await?;

		Ok(())
	}
}

impl LightNodeV1 {
	pub async fn tick_block_proposer(&self) -> Result<(), anyhow::Error> {
		let block = self.memseq.wait_for_next_block().await?;
		match block {
			Some(block) => {
				let block_blob = self.pass_through.create_new_celestia_blob(
					serde_json::to_vec(&block)
						.map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))?,
				)?;

				let height = self.pass_through.submit_celestia_blob(block_blob).await?;

				debug!("Submitted block: {:?} {:?}", block.id(), height);
			}
			None => {
				// no transactions to include
			}
		}
		Ok(())
	}

	pub async fn run_block_proposer(&self) -> Result<(), anyhow::Error> {
		loop {
			// build the next block from the blobs
			self.tick_block_proposer().await?;

			// sleep for a while
			tokio::time::sleep(std::time::Duration::from_millis(300)).await;
		}

		Ok(())
	}

	pub fn to_sequenced_blob_block(
		blob_response: BlobResponse,
	) -> Result<BlobResponse, anyhow::Error> {
		let blob_type = blob_response.blob_type.ok_or(anyhow::anyhow!("No blob type"))?;

		let sequenced_block = match blob_type {
			blob_response::BlobType::PassedThroughBlob(blob) => {
				blob_response::BlobType::SequencedBlobBlock(blob)
			}
			blob_response::BlobType::SequencedBlobBlock(blob) => {
				blob_response::BlobType::SequencedBlobBlock(blob)
			}
			_ => {
				anyhow::bail!("Invalid blob type")
			}
		};

		Ok(BlobResponse { blob_type: Some(sequenced_block) })
	}

	pub fn make_sequenced_blob_intent(
		data: Vec<u8>,
		height: u64,
	) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse {
			blob_type: Some(blob_response::BlobType::SequencedBlobIntent(Blob {
				data,
				blob_id: "".to_string(),
				height,
				timestamp: 0,
			})),
		})
	}
}

#[tonic::async_trait]
impl LightNodeService for LightNodeV1 {
	/// Server streaming response type for the StreamReadFromHeight method.
	type StreamReadFromHeightStream = std::pin::Pin<
		Box<
			dyn Stream<Item = Result<StreamReadFromHeightResponse, tonic::Status>> + Send + 'static,
		>,
	>;

	/// Stream blobs from a specified height or from the latest height.
	async fn stream_read_from_height(
		&self,
		request: tonic::Request<StreamReadFromHeightRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadFromHeightStream>, tonic::Status> {
		self.pass_through.stream_read_from_height(request).await
	}

	/// Server streaming response type for the StreamReadLatest method.
	type StreamReadLatestStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>,
	>;

	/// Stream the latest blobs.
	async fn stream_read_latest(
		&self,
		request: tonic::Request<StreamReadLatestRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadLatestStream>, tonic::Status> {
		self.pass_through.stream_read_latest(request).await
	}
	/// Server streaming response type for the StreamWriteCelestiaBlob method.
	type StreamWriteBlobStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamWriteBlobResponse, tonic::Status>> + Send + 'static>,
	>;
	/// Stream blobs out, either individually or in batches.
	async fn stream_write_blob(
		&self,
		request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		unimplemented!("stream_write_blob")
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		self.pass_through.read_at_height(request).await
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		self.pass_through.batch_read(request).await
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let blobs_for_intent = request.into_inner().blobs;
		let blobs_for_submission = blobs_for_intent.clone();
		let height: u64 = self
			.pass_through
			.default_client
			.header_network_head()
			.await
			.map_err(|e| tonic::Status::internal(e.to_string()))?
			.height()
			.into();

		let intents: Vec<BlobResponse> = blobs_for_intent
			.into_iter()
			.map(|blob| {
				Self::make_sequenced_blob_intent(blob.data, height)
					.map_err(|e| tonic::Status::internal(e.to_string()))
			})
			.collect::<Result<Vec<BlobResponse>, tonic::Status>>()?;

		// make transactions from the blobs
		let transactions: Vec<Transaction> = blobs_for_submission
			.into_iter()
			.map(|blob| {
				let transaction = Transaction::from(blob.data);
				transaction
			})
			.collect();

		// publish the transactions
		for transaction in transactions {
			debug!("Publishing transaction: {:?}", transaction.id());

			self.memseq
				.publish(transaction)
				.await
				.map_err(|e| tonic::Status::internal(e.to_string()))?;
		}

		Ok(tonic::Response::new(BatchWriteResponse { blobs: intents }))
	}
	/// Update and manage verification parameters.
	async fn update_verification_parameters(
		&self,
		request: tonic::Request<UpdateVerificationParametersRequest>,
	) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
		self.pass_through.update_verification_parameters(request).await
	}
}
