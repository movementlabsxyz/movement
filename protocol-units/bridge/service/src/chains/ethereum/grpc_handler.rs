use alloy::primitives::{keccak256, Address};
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

	#[cfg(test)]
	async fn new_for_test() -> Self {
		use super::client::Config;

		Self {
			eth_client: EthClient::new(Config::build_for_test())
				.await
				.expect("Failed to create EthClient"),
		}
	}
}

#[tonic::async_trait]
impl GRPCServer {
	// Verifies the signature for each request
	fn verify_signature_request(&self, message: &[u8], signature: &[u8]) -> Result<(), Status> {
		self.verify_signature(message, signature)
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

		// Verify signature
		self.verify_signature(
			// construct a message
			&[req.recipient_address.clone(), req.hash_lock.clone()].concat(),
			&req.signature,
		)?;

		// Call the EthClient method to initiate the bridge transfer
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

		// Verify signature
		self.verify_signature(
			&[req.bridge_transfer_id.clone(), req.pre_image.clone()].concat(),
			&req.signature,
		)?;

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

		// Verify signature
		self.verify_signature(
			&[req.bridge_transfer_id.clone(), req.pre_image.clone()].concat(),
			&req.signature,
		)?;

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

		// Verify signature
		self.verify_signature(&req.bridge_transfer_id, &req.signature)?;

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

		// Verify signature
		self.verify_signature(
			&[req.bridge_transfer_id.clone(), req.hash_lock.clone(), req.initiator_address.clone()]
				.concat(),
			&req.signature,
		)?;

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

		// Verify signature
		self.verify_signature_request(&req.bridge_transfer_id, &req.signature)?;

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

		// Verify signature
		self.verify_signature(&req.bridge_transfer_id, &req.signature)?;

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
				hash_lock: details.hash_lock.0.try_into().expect("Failed to convert hash lock"),
				time_lock: details.time_lock.0,
				amount: details.amount.value(),
				state: details.state as u32,
				error_message: "".to_string(),
			})),
			Ok(_) => Ok(Response::new(BridgeTransferDetailsResponse {
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

		// Verify signature
		self.verify_signature(&req.bridge_transfer_id, &req.signature)?;

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
				hash_lock: details.hash_lock.0.try_into().expect("Failed to convert hash lock"),
				time_lock: details.time_lock.0,
				amount: details.amount.value(),
				state: details.state as u32,
				error_message: "".to_string(),
			})),
			Ok(_) => Ok(Response::new(BridgeTransferDetailsResponse {
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

impl GRPCServer {
	fn verify_signature(&self, message: &[u8], signature_bytes: &[u8]) -> Result<(), Status> {
		// Hash the message using EIP-191 prefix and Keccak256 (Ethereum's prefixed message hash)
		let message_hash = ethers::utils::hash_message(message);

		// Parse the signature into the ethers Signature type
		let signature = ethers::types::Signature::try_from(signature_bytes)
			.map_err(|_| Status::unauthenticated("Invalid signature"))?;

		// Recover the Ethereum address from the signature
		let recovered_address = signature
			.recover(message_hash)
			.map_err(|_| Status::unauthenticated("Failed to recover public key"))?;

		// Convert the recovered ethers Address (H160) to alloy_primitives::Address
		let recovered_alloy_address =
			alloy::primitives::Address::from_slice(recovered_address.as_bytes());

		// Get the expected signer address from EthClient (alloy_primitives::Address)
		let expected_alloy_address = self.eth_client.get_signer_address();

		// Compare the recovered address with the expected address
		if recovered_alloy_address != expected_alloy_address {
			return Err(Status::unauthenticated("Signature does not match the expected signer"));
		}

		Ok(())
	}
}
