use bridge_grpc::{
	bridge_server::Bridge, AbortBridgeTransferRequest, BridgeTransferDetailsResponse,
	CounterpartyCompleteBridgeTransferRequest, GenericBridgeResponse,
	GetBridgeTransferDetailsRequest, InitiateBridgeTransferRequest, InitiateBridgeTransferResponse,
	InitiatorCompleteBridgeTransferRequest, LockBridgeTransferRequest, RefundBridgeTransferRequest,
};
use tonic::{Request, Response, Status};

use crate::{
	chains::bridge_contracts::BridgeContract,
	types::{Amount, AssetType, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage},
};

use super::{client::EthClient, types::EthAddress};

// Define the service struct that wraps EthClient
pub struct GRPCServer {
	eth_client: EthClient,
}

impl GRPCServer {
	pub fn new(eth_client: EthClient) -> Self {
		Self { eth_client }
	}
}

#[tonic::async_trait]
impl Bridge for GRPCServer {
	// Initiate bridge transfer
	async fn initiate_bridge_transfer(
		&self,
		request: Request<InitiateBridgeTransferRequest>,
	) -> Result<Response<InitiateBridgeTransferResponse>, Status> {
		let req = request.into_inner();

		match <EthClient as BridgeContract<EthAddress>>::initiate_bridge_transfer(
			&self.eth_client,
			BridgeAddress(EthAddress(self.eth_client.get_signer_address())),
			BridgeAddress(req.recipient_address),
			HashLock(req.hash_lock.try_into().unwrap()),
			Amount(AssetType::Moveth(req.amount)),
		)
		.await
		{
			Ok(_) => Ok(Response::new(InitiateBridgeTransferResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(InitiateBridgeTransferResponse {
				success: false,
				error_message: format!("Failed to initiate bridge transfer: {}", e),
			})),
		}
	}

	// Complete initiator bridge transfer
	async fn initiator_complete_bridge_transfer(
		&self,
		request: Request<InitiatorCompleteBridgeTransferRequest>,
	) -> Result<Response<GenericBridgeResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());
		let pre_image = HashLockPreImage(req.pre_image.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::initiator_complete_bridge_transfer(
			&self.eth_client,
			bridge_transfer_id,
			pre_image,
		)
		.await
		{
			Ok(_) => Ok(Response::new(GenericBridgeResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(GenericBridgeResponse {
				success: false,
				error_message: format!("Failed to complete bridge transfer: {}", e),
			})),
		}
	}

	// Complete counterparty bridge transfer
	async fn counterparty_complete_bridge_transfer(
		&self,
		request: Request<CounterpartyCompleteBridgeTransferRequest>,
	) -> Result<Response<GenericBridgeResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());
		let pre_image = HashLockPreImage(req.pre_image.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::counterparty_complete_bridge_transfer(
			&self.eth_client,
			bridge_transfer_id,
			pre_image,
		)
		.await
		{
			Ok(_) => Ok(Response::new(GenericBridgeResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(GenericBridgeResponse {
				success: false,
				error_message: format!("Failed to complete bridge transfer: {}", e),
			})),
		}
	}

	// Refund bridge transfer
	async fn refund_bridge_transfer(
		&self,
		request: Request<RefundBridgeTransferRequest>,
	) -> Result<Response<GenericBridgeResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::refund_bridge_transfer(
			&self.eth_client,
			bridge_transfer_id,
		)
		.await
		{
			Ok(_) => Ok(Response::new(GenericBridgeResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(GenericBridgeResponse {
				success: false,
				error_message: format!("Failed to refund bridge transfer: {}", e),
			})),
		}
	}

	// Lock bridge transfer
	async fn lock_bridge_transfer(
		&self,
		request: Request<LockBridgeTransferRequest>,
	) -> Result<Response<GenericBridgeResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());
		let hash_lock = HashLock(req.hash_lock.try_into().unwrap());
		let initiator = BridgeAddress(req.initiator_address.try_into().unwrap());
		let recipient = BridgeAddress(EthAddress(req.recipient_address.parse().unwrap()));
		let amount = Amount(AssetType::Moveth(req.amount));

		match <EthClient as BridgeContract<EthAddress>>::lock_bridge_transfer(
			&self.eth_client,
			bridge_transfer_id,
			hash_lock,
			initiator,
			recipient,
			amount,
		)
		.await
		{
			Ok(_) => Ok(Response::new(GenericBridgeResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(GenericBridgeResponse {
				success: false,
				error_message: format!("Failed to lock bridge transfer: {}", e),
			})),
		}
	}

	// Abort bridge transfer
	async fn abort_bridge_transfer(
		&self,
		request: Request<AbortBridgeTransferRequest>,
	) -> Result<Response<GenericBridgeResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::abort_bridge_transfer(
			&self.eth_client,
			bridge_transfer_id,
		)
		.await
		{
			Ok(_) => Ok(Response::new(GenericBridgeResponse {
				success: true,
				error_message: "".to_string(),
			})),
			Err(e) => Ok(Response::new(GenericBridgeResponse {
				success: false,
				error_message: format!("Failed to abort bridge transfer: {}", e),
			})),
		}
	}

	// Get details of initiator bridge transfer
	async fn get_bridge_transfer_details_initiator(
		&self,
		request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::get_bridge_transfer_details_initiator(
			&self.eth_client,
			bridge_transfer_id,
		)
		.await
		{
			Ok(Some(details)) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: details.initiator_address.0.to_string(),
				recipient_address: details.recipient_address.0,
				hash_lock: details.hash_lock.0,
				time_lock: details.time_lock.0,
				amount: details.amount.value(),
				state: details.state,
				error_message: "".to_string(),
			})),
			Ok(None) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: "".to_string(),
				recipient_address: vec![],
				hash_lock: vec![],
				time_lock: 0,
				amount: 0,
				state: 0,
				error_message: "No details found".to_string(),
			})),
			Err(e) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: "".to_string(),
				recipient_address: vec![],
				hash_lock: vec![],
				time_lock: 0,
				amount: 0,
				state: 0,
				error_message: format!("Failed to get bridge transfer details: {}", e),
			})),
		}
	}

	// Get details of counterparty bridge transfer
	async fn get_bridge_transfer_details_counterparty(
		&self,
		request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		let req = request.into_inner();

		let bridge_transfer_id = BridgeTransferId(req.bridge_transfer_id.try_into().unwrap());

		match <EthClient as BridgeContract<EthAddress>>::get_bridge_transfer_details_counterparty(
			&self.eth_client,
			bridge_transfer_id,
		)
		.await
		{
			Ok(Some(details)) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: details.initiator_address.0.to_string(),
				recipient_address: details.recipient_address.0,
				hash_lock: details.hash_lock.0,
				time_lock: details.time_lock.0,
				amount: details.amount.value(),
				state: details.state,
				error_message: "".to_string(),
			})),
			Ok(None) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: "".to_string(),
				recipient_address: vec![],
				hash_lock: vec![],
				time_lock: 0,
				amount: 0,
				state: 0,
				error_message: "No details found".to_string(),
			})),
			Err(e) => Ok(Response::new(BridgeTransferDetailsResponse {
				initiator_address: "".to_string(),
				recipient_address: vec![],
				hash_lock: vec![],
				time_lock: 0,
				amount: 0,
				state: 0,
				error_message: format!("Failed to get bridge transfer details: {}", e),
			})),
		}
	}
}
