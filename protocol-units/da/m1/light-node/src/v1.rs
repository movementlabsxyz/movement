use m1_da_light_node_grpc::light_node_service_server::LightNodeService;
use m1_da_light_node_grpc::*;
use tokio_stream::{StreamExt, Stream};
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{blob::GasPrice, nmt::Namespace, Blob as CelestiaBlob};
use std::sync::Arc;
use tokio::sync::RwLock;
use m1_da_light_node_util::Config;
use m1_da_light_node_verifier::{
    Verifier,
    v1::V1Verifier
};
use tempfile::tempdir;
use std::collections::HashMap;
use memseq::{Sequencer, Transaction};

#[cfg(feature = "sequencer")]
#[derive(Debug, Clone)]
pub struct BlockInclusion {
    pub submitted_timestamp : u64,
    pub included_height : Option<u64>
}

#[derive(Clone)]
pub struct LightNodeV1 {
    pub celestia_url : String,
    pub celestia_token : String,
    pub celestia_namespace : Namespace,
    pub default_client : Arc<Client>,
    pub verification_mode : Arc<RwLock<VerificationMode>>,
    pub verifier : Arc<Box<dyn Verifier + Send + Sync>>,

    #[cfg(feature = "sequencer")]
    pub memseq : Arc<memseq::Memseq<memseq::RocksdbMempool>>,

    // the block heights at which recent transactions were published
    #[cfg(feature = "sequencer")]
    pub block_heights : Arc<RwLock<
        HashMap<Vec<u8>, BlockInclusion>>
    >,
}

impl LightNodeV1 {

    /// Tries to create a new LightNodeV1 instance from the environment variables.
    pub async fn try_from_env() -> Result<Self, anyhow::Error> {

        let config = Config::try_from_env()?;
        let client = Arc::new(config.connect_celestia().await?);

        #[cfg(feature = "sequencer")]
        let memseq = {
            // generate temp dir
            let temp_dir = tempdir()?;
            let path = temp_dir.path().to_path_buf();
            Arc::new(memseq::Memseq::try_move_rocks(path)?)
        };
       
        Ok(Self {
            celestia_url: config.celestia_url,
            celestia_token: config.celestia_token,
            celestia_namespace: config.celestia_namespace,
            default_client: client.clone(),
            verification_mode: Arc::new(RwLock::new(config.verification_mode)),
            verifier: Arc::new(Box::new(V1Verifier {
                client: client,
                namespace: config.celestia_namespace.clone(),
            })),
            #[cfg(feature = "sequencer")]
            memseq,
            #[cfg(feature = "sequencer")]
            block_heights: Arc::new(RwLock::new(HashMap::new())),
        })
    

    }

    /// Gets a new Celestia client instance with the matching params. 
    pub async fn get_new_celestia_client(&self) -> Result<Client, anyhow::Error> {
        Client::new(&self.celestia_url, Some(&self.celestia_token))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Celestia client: {}", e))
    }

    /// Creates a new blob instance with the provided data.
    pub fn create_new_celestia_blob(&self, data: Vec<u8>) -> Result<CelestiaBlob, anyhow::Error> {
        CelestiaBlob::new(self.celestia_namespace, data)
            .map_err(|e| anyhow::anyhow!("Failed to create a blob: {}", e))
    }

    /// Submits a CelestiaNlob to the Celestia node.
    pub async fn submit_celestia_blob(&self, blob: CelestiaBlob) -> Result<u64, anyhow::Error> {
        let height = self.default_client.blob_submit(&[blob], GasPrice::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed submitting the blob: {}", e))?;

        Ok(height)
    }

    /// Submits a blob to the Celestia node.
    #[cfg(not(feature = "sequencer"))]
    pub async fn submit_blob(&self, data: Vec<u8>) -> Result<Blob, anyhow::Error> {
        let celestia_blob = self.create_new_celestia_blob(data)?;
        let height = self.submit_celestia_blob(celestia_blob.clone()).await?;
        Ok(Self::celestia_blob_to_blob(celestia_blob, height)?)
    }

    /// Submits blob to the sequencer which can then be published to the Celestia node. 
    #[cfg(feature = "sequencer")]
    pub async fn submit_blob(&self, data: Vec<u8>) -> Result<Blob, anyhow::Error> {
        let transaction = Transaction::new(data.clone());
        self.memseq.publish(transaction).await?;

        // todo: this is invalid
        // todo: we need to decide whether to invalidate the returning of blobs in sequencer mode
        // todo: or else we would want to wait for the actual block to be published and return that
        let celestia_blob = self.create_new_celestia_blob(data)?;
        Ok(Self::celestia_blob_to_blob(celestia_blob, 0)?) 
    }

    #[cfg(feature = "sequencer")]
    pub async fn tick_block_proposer(&self) -> Result<(), anyhow::Error> {
        let block = self.memseq.wait_for_next_block().await?;
        match block {
            Some(block) => {

                let block_blob = self.create_new_celestia_blob(
                    serde_json::to_vec(&block).map_err(
                        |e| anyhow::anyhow!("Failed to serialize block: {}", e)
                    )?
                )?;

                self.submit_celestia_blob(block_blob).await?;
            },
            None => {
                // no transactions to include
            }
        }
        Ok(())
    }

    #[cfg(feature = "sequencer")]
    pub async fn run_block_proposer(&self) -> Result<(), anyhow::Error> {
        loop {
            // build the next block from the blobs
            self.tick_block_proposer().await?;

            // sleep for a while
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        Ok(())
    }

    /// Gets the blobs at a given height.
    pub async fn get_celestia_blobs_at_height(&self, height: u64) -> Result<Vec<CelestiaBlob>, anyhow::Error> {

        let blobs = self.default_client
            .blob_get_all(height, &[self.celestia_namespace])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blobs at height: {}", e))?;

        let mut verified_blobs = Vec::new();
        for blob in blobs {

            let blob_data = blob.data.clone();

            // todo: improve error boundary here to detect crashes
            let verified = self.verifier.verify(
                *self.verification_mode.read().await,
                &blob_data,
                height,
            ).await.is_ok_and(|v| v);

            if verified {
                verified_blobs.push(blob);
            }

        }

        Ok(verified_blobs)

    }

    pub async fn get_blobs_at_height(&self, height: u64) -> Result<Vec<Blob>, anyhow::Error> {
        let celestia_blobs = self.get_celestia_blobs_at_height(height).await?;
        let mut blobs = Vec::new();
        for blob in celestia_blobs {
            blobs.push(Self::celestia_blob_to_blob(blob, height)?);
        }
        Ok(blobs)
    }

    /// Streams blobs until it can't get another one in the loop
    pub async fn stream_blobs_in_range(&self, start_height : u64, end_height : Option<u64>) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>, anyhow::Error> {

        let mut height = start_height;
        let end_height = end_height.unwrap_or_else(|| u64::MAX);
        let me = Arc::new(self.clone());

        let stream = async_stream::try_stream! {
            loop {
                if height > end_height {
                    break;
                }

                let blobs = me.get_blobs_at_height(height).await?;
                for blob in blobs {
                    yield blob;
                }
                height += 1;
            }
        };

        Ok(Box::pin(stream) as std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>)
    }

    /// Streams the latest blobs that can subscribed to.
    async fn stream_blobs_from_height_on(&self, start_height : Option<u64>) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>, anyhow::Error> {
        let start_height = start_height.unwrap_or_else(|| u64::MAX);
        let me = Arc::new(self.clone());
        let mut subscription = me.default_client.header_subscribe().await?;

        let stream = async_stream::try_stream! {
            let mut first_flag = true;
            while let Some(header_res) = subscription.next().await {

                let header = header_res?;
                let height = header.height().into();

                // back fetch the blobs
                if first_flag && height > start_height {
        
                    let mut blob_stream = me.stream_blobs_in_range(start_height, Some(height)).await?;
                    
                    while let Some(blob) = blob_stream.next().await {
                        yield blob?;
                    }

                }
                first_flag = false;

                let blobs = me.get_blobs_at_height(height).await?;
                for blob in blobs {
                    yield blob;
                }
            }
        };

        Ok(Box::pin(stream) as std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>)
    }

    pub fn celestia_blob_to_blob(blob: CelestiaBlob, height: u64) -> Result<Blob, anyhow::Error> {
        Ok(Blob {
            data: blob.data,
            blob_id: serde_json::to_string(&blob.commitment).map_err(
                |e| anyhow::anyhow!("Failed to serialize commitment: {}", e)
            )?,
            height
        })
    }
        
}

#[tonic::async_trait]
impl LightNodeService for LightNodeV1 {

    /// Server streaming response type for the StreamReadFromHeight method.
    type StreamReadFromHeightStream = std::pin::Pin<Box<dyn Stream<Item = Result<StreamReadFromHeightResponse, tonic::Status>> + Send + 'static>>;

    /// Stream blobs from a specified height or from the latest height.
    async fn stream_read_from_height(
        &self,
        request: tonic::Request<StreamReadFromHeightRequest>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadFromHeightStream>,
        tonic::Status,
    > {
        let me = Arc::new(self.clone());
        let height = request.into_inner().height;

        let output = async_stream::try_stream! {

            let mut blob_stream = me.stream_blobs_from_height_on(Some(height)).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

            while let Some(blob) = blob_stream.next().await {
                let blob = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
                let response = StreamReadFromHeightResponse {
                    blob : Some(blob)
                };
                yield response;
            }
            
        };

        Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadFromHeightStream))
    }

    /// Server streaming response type for the StreamReadLatest method.
    type StreamReadLatestStream = std::pin::Pin<Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>>;

    /// Stream the latest blobs.
    async fn stream_read_latest(
        &self,
        _request: tonic::Request<StreamReadLatestRequest>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadLatestStream>,
        tonic::Status,
    > {
        let me = Arc::new(self.clone());

        let output = async_stream::try_stream! {
            
            let mut blob_stream = me.stream_blobs_from_height_on(None).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
            while let Some(blob) = blob_stream.next().await {
                let blob = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
                let response = StreamReadLatestResponse {
                    blob : Some(blob)
                };
                yield response;
            }
        
        };

        Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadLatestStream))
        
    }
    /// Server streaming response type for the StreamWriteCelestiaBlob method.
    type StreamWriteBlobStream = std::pin::Pin<Box<dyn Stream<Item = Result<StreamWriteBlobResponse, tonic::Status>> + Send + 'static>>;
    /// Stream blobs out, either individually or in batches.
    async fn stream_write_blob(
        &self,
        request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamWriteBlobStream>,
        tonic::Status,
    > {

        let mut stream = request.into_inner();
        let me = Arc::new(self.clone());
    
        let output = async_stream::try_stream! {
            // Note: using try_stream! here was replaced with stream! for illustration, handling errors should be adapted
            while let Some(request) = stream.next().await {
                let request = request?;
                let blob_data = request.blob.ok_or(tonic::Status::invalid_argument("No blob in request"))?.data;

                let blob = me.submit_blob(blob_data).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
               
                let write_response = StreamWriteBlobResponse {
                    blob : Some(blob)
                };

                yield write_response;

            }
        };
    
        Ok(tonic::Response::new(Box::pin(output) as Self::StreamWriteBlobStream))

    }
    /// Read blobs at a specified height.
    async fn read_at_height(
        &self,
        request: tonic::Request<ReadAtHeightRequest>,
    ) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
        
        let height = request.into_inner().height;
        let blobs = self.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

        if blobs.is_empty() {
            return Err(tonic::Status::not_found("No blobs found at the specified height"));
        }

        Ok(tonic::Response::new(ReadAtHeightResponse {
            blobs
        }))

    }
    /// Batch read and write operations for efficiency.
    async fn batch_read(
        &self,
        request: tonic::Request<BatchReadRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchReadResponse>,
        tonic::Status,
    > {
        let heights = request.into_inner().heights;
        let mut responses = Vec::with_capacity(heights.len());
        for height in heights {

            let blobs = self.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

            if blobs.is_empty() {
                return Err(tonic::Status::not_found("No blobs found at the specified height"));
            }

            responses.push(ReadAtHeightResponse {
                blobs
            })
    
        }

        Ok(tonic::Response::new(BatchReadResponse {
            responses
        }))

    }

    /// Batch write blobs.
    async fn batch_write(
        &self,
        request: tonic::Request<BatchWriteRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchWriteResponse>,
        tonic::Status,
    > {
        let blobs = request.into_inner().blobs; 
        let mut responses = Vec::with_capacity(blobs.len());
        for data in blobs { 
            let blob = self.submit_blob(data.data).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
            responses.push(blob);
        }

        Ok(tonic::Response::new(BatchWriteResponse {
            blobs : responses
        }))

    }
    /// Update and manage verification parameters.
    async fn update_verification_parameters(
        &self,
        request: tonic::Request<UpdateVerificationParametersRequest>,
    ) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
            
        let verification_mode = request.into_inner().mode();
        let mut mode = self.verification_mode.write().await;
        *mode = verification_mode;

        Ok(tonic::Response::new(UpdateVerificationParametersResponse {
            mode : verification_mode.into()
        }))

    }

}