use m1_da_light_node_grpc::light_node_server::LightNode;
use m1_da_light_node_grpc::*;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug, Default)]
pub struct LightNodeV1 {}

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
    type StreamWriteBlobStream = ReceiverStream<Result<WriteResponse, tonic::Status>>;
    /// Stream blobs out, either individually or in batches.
    async fn stream_write_blob(
        &self,
        request: tonic::Request<tonic::Streaming<BlobWriteRequest>>,
    ) -> std::result::Result<
        tonic::Response<Self::StreamWriteBlobStream>,
        tonic::Status,
    > {
        unimplemented!()
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