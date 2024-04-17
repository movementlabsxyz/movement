use m1_da_light_node_grpc::light_node_server::LightNode;
use m1_da_light_node_grpc::*;
use tokio_stream::{wrappers::ReceiverStream, StreamExt, Stream};
use celestia_rpc::{BlobClient, Client};
use celestia_types::{blob::GasPrice, nmt::Namespace, Blob};
use std::sync::Arc;

#[derive(Clone)]
pub struct LightNodeV1 {
    // pub celestia_client : Client,
    pub celestia_url : String,
    pub celestia_token : String,
    pub celestia_namespace : Namespace,
    pub default_client : Arc<Client>
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

        Ok(Self {
            celestia_url: url,
            celestia_token: token,
            celestia_namespace: namespace,
            default_client: Arc::new(client)
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
    pub async fn submit_blob(&self, blob: Blob) -> Result<(), anyhow::Error> {
        self.default_client.blob_submit(&[blob], GasPrice::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed submitting the blob: {}", e))?;
        Ok(())
    }
        
}

#[tonic::async_trait]
impl LightNode for LightNodeV1 {

    /// Server streaming response type for the StreamReadFromHeight method.
    type StreamReadFromHeightStream = ReceiverStream<Result<BlobResponse, tonic::Status>>;

    /// Stream blobs from a specified height or from the latest height.
    async fn stream_read_from_height(
        &self,
        request: tonic::Request<StreamReadRequest>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadFromHeightStream>,
        tonic::Status,
    > {
        unimplemented!()
    }
    /// Server streaming response type for the StreamReadLatest method.
    type StreamReadLatestStream = ReceiverStream<Result<BlobResponse, tonic::Status>>;
    async fn stream_read_latest(
        &self,
        request: tonic::Request<Empty>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadLatestStream>,
        tonic::Status,
    > {
        unimplemented!()
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
                let blob = me.create_new_blob(request.data).map_err(|e| tonic::Status::internal(e.to_string()))?;
    
                // Submitting the blob, handle errors appropriately
                me.submit_blob(blob).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
    
                let write_response = WriteResponse {
                    success: true,
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
    ) -> std::result::Result<tonic::Response<BlobResponse>, tonic::Status> {
        unimplemented!()
    }
    /// Batch read and write operations for efficiency.
    async fn batch_read(
        &self,
        request: tonic::Request<BatchReadRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchReadResponse>,
        tonic::Status,
    > {
        unimplemented!()
    }
    async fn batch_write(
        &self,
        request: tonic::Request<BatchWriteRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchWriteResponse>,
        tonic::Status,
    > {
        unimplemented!()
    }
    /// Update and manage verification parameters.
    async fn update_verification_parameters(
        &self,
        request: tonic::Request<VerificationParametersRequest>,
    ) -> std::result::Result<tonic::Response<UpdateResponse>, tonic::Status> {
        unimplemented!()
    }

}