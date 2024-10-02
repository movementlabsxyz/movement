use super::{client::EthClient, types::EthAddress};
use crate::{chains::bridge_contracts::BridgeContract, types::BridgeTransferId};
use bridge_grpc::{
	bridge_server::Bridge, BridgeTransferDetailsResponse, GetBridgeTransferDetailsRequest,
};
use tonic::{Request, Response, Status};

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

// More Admin only utility functions can be added to get more information.
// For example list of ongoing tasks, list of completed tasks, etc.

#[tonic::async_trait]
impl Bridge for GRPCServer {
	// Get details of initiator bridge transfer
	async fn get_bridge_transfer_details_initiator(
		&self,
		request: Request<GetBridgeTransferDetailsRequest>,
	) -> Result<Response<BridgeTransferDetailsResponse>, Status> {
		let req = request.into_inner();

		//gRPC methods are protected
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
				hash_lock: details.hash_lock.0.into(),
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
				hash_lock: details.hash_lock.0.into(),
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
