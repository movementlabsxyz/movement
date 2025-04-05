use futures_util::StreamExt;
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
use std::time::Duration;
use tokio::sync::mpsc;
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

#[tokio::test]
async fn test_client_reconnect_if_connection_fails() {
	// Bind to an available port but do not start the server yet
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let url = format!("http://{}", addr);

	// Begin trying to connect to the DA server before it's running
	let client_task = tokio::spawn(async move { DaSequencerClient::try_connect(&url).await });

	// Simulate the server being offline briefly
	tokio::time::sleep(Duration::from_secs(2)).await;

	// Now start the server
	tokio::spawn(async move {
		let service = DaSequencerNodeServiceServer::new(MockService);
		Server::builder()
			.add_service(service)
			.serve_with_incoming(TcpListenerStream::new(listener))
			.await
			.unwrap();
	});

	// The client should eventually succeed
	let result = client_task.await.unwrap();
	assert!(result.is_ok(), "Expected client to reconnect after retries, but it failed");
}

#[tokio::test]
async fn test_stream_reconnects_and_resumes_from_correct_height() {
	use std::sync::{Arc, Mutex};
	use tokio::sync::{mpsc, oneshot};

	// Shared state for sending blocks across server restarts
	let _blocks_sent = Arc::new(Mutex::new(vec![
		0, 1, // first server sends blocks 0 and 1
		2, 3, // second server sends blocks 2 and 3
	]));

	// Mock service that streams blocks based on the current `blocks_sent`
	struct ReconnectableMock {
		heights: Arc<Mutex<Vec<u64>>>,
	}

	#[tonic::async_trait]
	impl DaSequencerNodeService for ReconnectableMock {
		type StreamReadFromHeightStream =
			ReceiverStream<Result<StreamReadFromHeightResponse, Status>>;

		async fn stream_read_from_height(
			&self,
			request: Request<StreamReadFromHeightRequest>,
		) -> Result<Response<Self::StreamReadFromHeightStream>, Status> {
			let start_height = request.into_inner().height;
			let (tx, rx) = mpsc::channel(10);

			let heights = self.heights.lock().unwrap().clone();
			tokio::spawn(async move {
				for h in heights.into_iter().filter(|h| *h >= start_height) {
					let blob = BlobResponse {
						blob_type: Some(BlobType::Blockv1(Blockv1 {
							blobckid: vec![],
							data: vec![h as u8],
							height: h,
						})),
					};

					let msg = StreamReadFromHeightResponse { response: Some(blob) };

					tx.send(Ok(msg)).await.unwrap();
					tokio::time::sleep(Duration::from_millis(100)).await;
				}
			});

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
			unimplemented!()
		}
	}

	let addr = "127.0.0.1:50055".parse::<SocketAddr>().unwrap();
	let url = format!("http://{}", addr);

	// First server: send blocks 0 and 1
	let heights_1 = Arc::new(Mutex::new(vec![0, 1]));
	let mock_1 = ReconnectableMock { heights: heights_1.clone() };

	let (shutdown_tx, shutdown_rx) = oneshot::channel();
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

	tokio::spawn(async move {
		Server::builder()
			.add_service(DaSequencerNodeServiceServer::new(mock_1))
			.serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
				shutdown_rx.await.ok();
			})
			.await
			.unwrap();
	});

	// Connect client and start receiving blocks
	let mut client = DaSequencerClient::try_connect(&url).await.unwrap();

	let mut last_height = 0;
	let mut stream = client
		.stream_read_from_height(StreamReadFromHeightRequest { height: 0 })
		.await
		.unwrap();

	// Receive first two blocks
	for _ in 0..2 {
		let res = stream.next().await.unwrap().unwrap();
		last_height = match res.response.unwrap().blob_type.unwrap() {
			movement_da_sequencer_proto::blob_response::BlobType::Blockv1(inner) => inner.height,
			_ => panic!("unexpected blob type"),
		};
	}

	// Shut down first server
	let _ = shutdown_tx.send(());
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Second server: send blocks 2 and 3
	let heights_2 = Arc::new(Mutex::new(vec![2, 3]));
	let mock_2 = ReconnectableMock { heights: heights_2.clone() };
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	tokio::spawn(async move {
		Server::builder()
			.add_service(DaSequencerNodeServiceServer::new(mock_2))
			.serve_with_incoming(TcpListenerStream::new(listener))
			.await
			.unwrap();
	});

	// Resume stream from last_height + 1
	let mut stream = client
		.stream_read_from_height(StreamReadFromHeightRequest { height: last_height + 1 })
		.await
		.unwrap();

	let res = stream.next().await.unwrap().unwrap();
	let new_height = match res.response.unwrap().blob_type.unwrap() {
		movement_da_sequencer_proto::blob_response::BlobType::Blockv1(inner) => inner.height,
		_ => panic!("unexpected blob type"),
	};
	assert_eq!(new_height, last_height + 1, "Client did not resume at last height + 1");
}
