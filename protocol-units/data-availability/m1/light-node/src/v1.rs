use m1_da_light_node_grpc::light_node_server::LightNode;
use m1_da_light_node_grpc::*;
use tokio_stream::{StreamExt, Stream};
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{blob::GasPrice, nmt::Namespace, Blob};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct LightNodeV1 {
    // pub celestia_client : Client,
    pub celestia_url : String,
    pub celestia_token : String,
    pub celestia_namespace : Namespace,
    pub default_client : Arc<Client>,
    pub verification_mode : Arc<RwLock<VerificationMode>>,
}

impl LightNodeV1 {

    const DEFAULT_CELESTIA_NODE_URL: &'static str = "ws://localhost:26658";
    const DEFAULT_NAMESPACE_BYTES: &'static str = "a673006fb64aa2e5360d";

    /// Tries to create a new LightNodeV1 instance from the environment variables.
    pub async fn try_from_env() -> Result<Self, anyhow::Error> {

        let token = std::env::var("CELESTIA_NODE_AUTH_TOKEN").map_err(
            |_| anyhow::anyhow!("Token not provided")
        )?; // expect("Token not provided"
        let url = std::env::var("CELESTIA_NODE_URL").unwrap_or_else(|_| Self::DEFAULT_CELESTIA_NODE_URL.to_string());
        
        
        let namespace_hex = std::env::var("CELESTIA_NAMESPACE_BYTES")
        .unwrap_or_else(|_| Self::DEFAULT_NAMESPACE_BYTES.to_string());

        // Decode the hex string to bytes
        let namespace_bytes = hex::decode(namespace_hex).map_err(|e| anyhow::anyhow!("Failed to decode namespace bytes: {}", e))?;

        // Create a namespace from the bytes
        let namespace = Namespace::new_v0(&namespace_bytes)?;

        let client = Client::new(&url, Some(&token)).await.map_err(|e| anyhow::anyhow!("Failed to create Celestia client: {}", e))?;

        // try to read the verification mode from the environment
        let verification_mode = match std::env::var("VERIFICATION_MODE") {
            Ok(mode) => {
              VerificationMode::from_str_name(mode.as_str()).ok_or(anyhow::anyhow!("Invalid verification mode"))?
            },
            Err(_) => VerificationMode::MOfN
        };

        Ok(Self {
            celestia_url: url,
            celestia_token: token,
            celestia_namespace: namespace,
            default_client: Arc::new(client),
            verification_mode: Arc::new(RwLock::new(verification_mode)),
        })
    

    }

    /// Gets a new Celestia client instance with the matching params. 
    pub async fn get_new_celestia_client(&self) -> Result<Client, anyhow::Error> {
        Client::new(&self.celestia_url, Some(&self.celestia_token))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Celestia client: {}", e))
    }

    /// Creates a new blob instance with the provided data.
    pub fn create_new_blob(&self, data: Vec<u8>) -> Result<Blob, anyhow::Error> {
        Blob::new(self.celestia_namespace, data)
            .map_err(|e| anyhow::anyhow!("Failed to create a blob: {}", e))
    }

    /// Submits a blob to the Celestia node.
    pub async fn submit_blob(&self, blob: Blob) -> Result<u64, anyhow::Error> {
        let height = self.default_client.blob_submit(&[blob], GasPrice::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed submitting the blob: {}", e))?;

        Ok(height)
    }

    /// Gets the blobs at a given height.
    pub async fn get_blobs_at_height(&self, height: u64) -> Result<Vec<Blob>, anyhow::Error> {

        self.default_client
            .blob_get_all(height, &[self.celestia_namespace])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get blobs at height: {}", e))
    }
        
}

#[tonic::async_trait]
impl LightNode for LightNodeV1 {

    /// Server streaming response type for the StreamReadFromHeight method.
    type StreamReadFromHeightStream = std::pin::Pin<Box<dyn Stream<Item = Result<BlobResponse, tonic::Status>> + Send + 'static>>;

    /// Stream blobs from a specified height or from the latest height.
    async fn stream_read_from_height(
        &self,
        request: tonic::Request<StreamReadRequest>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadFromHeightStream>,
        tonic::Status,
    > {
        let me = Arc::new(self.clone());
        let mut height = request.into_inner().start_height;

        let output = async_stream::try_stream! {

            loop {
                let blobs = me.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

                if blobs.is_empty() {
                    break;
                }

                for blob in blobs {
                    let blob_response = BlobResponse {
                        data : blob.data,
                        blob_id : serde_json::to_string(&blob.commitment).map_err(|e| tonic::Status::internal(e.to_string()))?,
                        height : height,
                    };
                    yield blob_response;
                }
                height += 1;
            }
            
        };

        Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadFromHeightStream))
    }

    /// Server streaming response type for the StreamReadLatest method.
    type StreamReadLatestStream = std::pin::Pin<Box<dyn Stream<Item = Result<BlobResponse, tonic::Status>> + Send + 'static>>;

    /// Stream the latest blobs.
    async fn stream_read_latest(
        &self,
        _request: tonic::Request<Empty>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadLatestStream>,
        tonic::Status,
    > {
        let me = Arc::new(self.clone());
        let mut subscription = me.default_client.header_subscribe().await.map_err(|e| tonic::Status::internal(e.to_string()))?;

        let output = async_stream::try_stream! {
            while let Some(header_res) = subscription.next().await {
                let header = header_res.map_err(|e| tonic::Status::internal(e.to_string()))?;    
                let height = header.height().into();
                let blobs = me.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

                for blob in blobs {
                    let blob_response = BlobResponse {
                        data : blob.data,
                        blob_id : serde_json::to_string(&blob.commitment).map_err(|e| tonic::Status::internal(e.to_string()))?,
                        height : height,
                    };
                    yield blob_response;
                }

            }
        };

        Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadLatestStream))
        
    }
    /// Server streaming response type for the StreamWriteBlob method.
    type StreamWriteBlobStream = std::pin::Pin<Box<dyn Stream<Item = Result<WriteResponse, tonic::Status>> + Send + 'static>>;
    /// Stream blobs out, either individually or in batches.
    async fn stream_write_blob(
        &self,
        request: tonic::Request<tonic::Streaming<BlobWriteRequest>>,
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
                // Process each request item
                let blob = me.create_new_blob(request.data.clone()).map_err(|e| tonic::Status::internal(e.to_string()))?;
    
                // Submitting the blob, handle errors appropriately
                let height = me.submit_blob(blob).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
    
                let blob_response = BlobResponse {
                    data : request.data.clone(),
                    blob_id : "".to_string(),
                    height : height,
                };
                let write_response = WriteResponse {
                    blob : Some(blob_response)
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
    ) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
        
        let height = request.into_inner().height;
        let blobs = self.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
        if blobs.is_empty() {
            return Err(tonic::Status::not_found("No blobs found at the specified height"));
        }

        let mut blob_responses = Vec::new();
        for blob in blobs {
            let blob_response = BlobResponse {
                data : blob.data,
                blob_id : serde_json::to_string(&blob.commitment).map_err(|e| tonic::Status::internal(e.to_string()))?,
                height : height,
            };
            blob_responses.push(blob_response);
        }
        let batch_react_response = BatchReadResponse {
            blobs : blob_responses
        };

        Ok(tonic::Response::new(batch_react_response))

    }
    /// Batch read and write operations for efficiency.
    async fn batch_read(
        &self,
        request: tonic::Request<BatchReadRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchReadResponse>,
        tonic::Status,
    > {
        
        let mut blob_responses = Vec::new();
        for height in request.into_inner().heights {
            let blobs = self.get_blobs_at_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
            if blobs.is_empty() {
                return Err(tonic::Status::not_found("No blobs found at the specified height"));
            }

    
            for blob in blobs {
                let blob_response = BlobResponse {
                    data : blob.data,
                    blob_id : serde_json::to_string(&blob.commitment).map_err(|e| tonic::Status::internal(e.to_string()))?,
                    height : height,
                };
                blob_responses.push(blob_response);
            }
    
        }

        Ok(tonic::Response::new(BatchReadResponse {
            blobs : blob_responses
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
        
        let mut blob_responses = Vec::new();
        for data in request.into_inner().data {
            let blob = self.create_new_blob(data.clone()).map_err(|e| tonic::Status::internal(e.to_string()))?;
            let height = self.submit_blob(blob).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
            let blob_response = BlobResponse {
                data : data,
                blob_id : "".to_string(),
                height : height,
            };
            blob_responses.push(blob_response);
        }

        Ok(tonic::Response::new(BatchWriteResponse {
            blobs : blob_responses
        }))

    }
    /// Update and manage verification parameters.
    async fn update_verification_parameters(
        &self,
        request: tonic::Request<VerificationParametersRequest>,
    ) -> std::result::Result<tonic::Response<UpdateResponse>, tonic::Status> {
            
            let verification_mode = request.into_inner().mode();
            let mut mode = self.verification_mode.write().await;
            *mode = verification_mode;
    
            Ok(tonic::Response::new(UpdateResponse {
                mode : verification_mode.into()
            }))
    }

}