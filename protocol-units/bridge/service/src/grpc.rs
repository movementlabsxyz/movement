use crate::chains::ethereum::client::EthClient;
use bridge_grpc::{
	bridge_server::Bridge, health_check_response::ServingStatus, health_server::Health,
	BridgeTransferDetailsResponse, GetBridgeTransferDetailsRequest, HealthCheckRequest,
	HealthCheckResponse,
};
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// A gRPC Health Check Service
#[derive(Default)]
pub struct HealthCheckService {
	status: std::sync::Mutex<std::collections::HashMap<String, ServingStatus>>,
}

// Define a stream that will be used for the Watch method
pub struct HealthWatchStream {
	receiver: mpsc::Receiver<Result<HealthCheckResponse, Status>>,
}

impl Stream for HealthWatchStream {
	type Item = Result<HealthCheckResponse, Status>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		self.receiver.poll_recv(cx)
	}
}

#[tonic::async_trait]
impl Health for HealthCheckService {
	type WatchStream = HealthWatchStream;

	async fn check(
		&self,
		request: Request<HealthCheckRequest>,
	) -> Result<Response<HealthCheckResponse>, Status> {
		let service = request.into_inner().service;
		let status_map = self.status.lock().unwrap();
		let status = status_map.get(&service).cloned().unwrap_or(ServingStatus::ServiceUnknown);

		Ok(Response::new(HealthCheckResponse { status: status.into() }))
	}

	async fn watch(
		&self,
		_request: Request<HealthCheckRequest>,
	) -> Result<Response<Self::WatchStream>, Status> {
		// Create an mpsc channel for the stream
		let (tx, rx) = mpsc::channel(4);
		let status_update = HealthCheckResponse { status: ServingStatus::Serving.into() };
		tx.send(Ok(status_update)).await.unwrap();

		Ok(Response::new(HealthWatchStream { receiver: rx }))
	}
}

impl HealthCheckService {
	// Set the health status of a service
	pub fn set_service_status(&self, service: &str, status: ServingStatus) {
		let mut status_map = self.status.lock().unwrap();
		status_map.insert(service.to_string(), status);
	}
}

/// Implement the gRPC service `Bridge` for the Ethereum Bridge Client
#[tonic::async_trait]
impl Bridge for EthClient {
	async fn get_bridge_transfer_details_initiator_eth(
		&self,
		_request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		unimplemented!()
	}

	async fn get_bridge_transfer_details_counterparty_eth(
		&self,
		_request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		unimplemented!()
	}

	async fn get_bridge_transfer_details_initiator_movement(
		&self,
		_request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		unimplemented!()
	}

	async fn get_bridge_transfer_details_counterparty_movement(
		&self,
		_request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		unimplemented!()
	}
}
