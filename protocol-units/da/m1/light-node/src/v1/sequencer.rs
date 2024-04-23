use m1_da_light_node_grpc::light_node_service_server::LightNodeService;
use m1_da_light_node_grpc::*;
use tokio_stream::Stream;
use crate::v1::pass_through::LightNodeV1;

pub struct LightNodeV1Sequencer {
    pub pass_through : LightNodeV1,
    pub sequencer : memseq::Memseq<memseq::RocksdbMempool>
}

impl LightNodeV1Sequencer {


}

#[tonic::async_trait]
impl LightNodeService for LightNodeV1Sequencer {

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
            
        self.pass_through.stream_read_from_height(request).await
    }

    /// Server streaming response type for the StreamReadLatest method.
    type StreamReadLatestStream = std::pin::Pin<Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>>;

    /// Stream the latest blobs.
    async fn stream_read_latest(
        &self,
        request: tonic::Request<StreamReadLatestRequest>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamReadLatestStream>,
        tonic::Status,
    > {
        
        self.pass_through.stream_read_latest(request).await
        
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

        unimplemented!("StreamWriteBlob not implemented")

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
    ) -> std::result::Result<
        tonic::Response<BatchReadResponse>,
        tonic::Status,
    > {
        self.pass_through.batch_read(request).await
    }

    /// Batch write blobs.
    async fn batch_write(
        &self,
        request: tonic::Request<BatchWriteRequest>,
    ) -> std::result::Result<
        tonic::Response<BatchWriteResponse>,
        tonic::Status,
    > {
       
        unimplemented!("StreamWriteBlob not implemented")

    }
    /// Update and manage verification parameters.
    async fn update_verification_parameters(
        &self,
        request: tonic::Request<UpdateVerificationParametersRequest>,
    ) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
            
       self.pass_through.update_verification_parameters(request).await

    }

}