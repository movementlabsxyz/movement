use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_proto::blob_response::BlobType;
use movement_da_sequencer_proto::da_sequencer_node_service_server::{
	DaSequencerNodeService, DaSequencerNodeServiceServer,
};
use movement_da_sequencer_proto::{
	BatchWriteRequest, BatchWriteResponse, BlobResponse, Blockv1, ReadAtHeightRequest,
	ReadAtHeightResponse, StreamReadFromHeightRequest, StreamReadFromHeightResponse,
};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

struct MockService;

#[tonic::async_trait]
impl DaSequencerNodeService for MockService {
	type StreamReadFromHeightStream = ReceiverStream<Result<StreamReadFromHeightResponse, Status>>;

	async fn stream_read_from_height(
		&self,
		_request: Request<StreamReadFromHeightRequest>,
	) -> Result<Response<Self::StreamReadFromHeightStream>, Status> {
		let (tx, rx) = mpsc::channel(1);

		let blob = BlobResponse {
			blob_type: Some(BlobType::Blockv1(Blockv1 {
				blobckid: vec![],
				data: vec![],
				height: 0,
			})),
		};

		let _ = tx.send(Ok(StreamReadFromHeightResponse { response: Some(blob) })).await;

		Ok(Response::new(ReceiverStream::new(rx)))
	}

	async fn batch_write(
		&self,
		_request: Request<BatchWriteRequest>,
	) -> Result<Response<BatchWriteResponse>, Status> {
		Ok(Response::new(BatchWriteResponse { answer: true }))
	}

	async fn read_at_height(
		&self,
		_request: Request<ReadAtHeightRequest>,
	) -> Result<Response<ReadAtHeightResponse>, Status> {
		let blob = BlobResponse {
			blob_type: Some(BlobType::Blockv1(Blockv1 {
				blobckid: vec![],
				data: vec![],
				height: 0,
			})),
		};

		Ok(Response::new(ReadAtHeightResponse { response: Some(blob) }))
	}
}

async fn start_mock_server_with_control() -> (SocketAddr, oneshot::Sender<()>) {
	let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	let bound_addr = listener.local_addr().unwrap();
	let (shutdown_tx, shutdown_rx) = oneshot::channel();

	let service = DaSequencerNodeServiceServer::new(MockService);
	tokio::spawn(async move {
		Server::builder()
			.add_service(service)
			.serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
				shutdown_rx.await.ok();
			})
			.await
			.unwrap();
	});

	(bound_addr, shutdown_tx)
}

#[tokio::test]
async fn test_client_reconnect_if_connection_fails() {
	let should_start_server = Arc::new(AtomicBool::new(false));
	let signal_server = should_start_server.clone();

	let (addr, shutdown_tx) = {
		let (tx, rx) = oneshot::channel();
		let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
		let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
		let bound_addr = listener.local_addr().unwrap();

		let service = DaSequencerNodeServiceServer::new(MockService);

		tokio::spawn(async move {
			rx.await.ok();
			Server::builder()
				.add_service(service)
				.serve_with_incoming(TcpListenerStream::new(listener))
				.await
				.unwrap();
		});

		(bound_addr, tx)
	};

	let url = format!("http://{}", addr);

	let client_task = tokio::spawn(async move { DaSequencerClient::try_connect(&url).await });

	// Wait before triggering the server to simulate retry
	tokio::time::sleep(Duration::from_secs(3)).await;
	signal_server.store(true, Ordering::Relaxed);
	let _ = shutdown_tx.send(());

	let result = client_task.await.unwrap();
	assert!(result.is_ok(), "Expected client to reconnect after retries, but it failed");
}

#[tokio::test]
async fn test_reopen_block_stream_at_correct_height() {
	let (addr, shutdown_tx) = start_mock_server_with_control().await;
	let url = format!("http://{}", addr);

	let mut client = DaSequencerClient::try_connect(&url).await.expect("Failed to connect");

	let request = StreamReadFromHeightRequest { height: 0 };
	let stream_result = client.stream_read_from_height(request).await;
	assert!(stream_result.is_ok());

	let _ = shutdown_tx.send(());
}
