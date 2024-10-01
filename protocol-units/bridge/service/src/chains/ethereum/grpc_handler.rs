use bridge_grpc::{
	bridge_server::Bridge, InitiateBridgeTransferRequest, InitiateBridgeTransferResponse,
};
use tonic::{Request, Response, Status};

use crate::{
	chains::bridge_contracts::BridgeContract,
	types::{Amount, AssetType, BridgeAddress, HashLock},
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
	async fn initiate_bridge_transfer(
		&self,
		request: Request<InitiateBridgeTransferRequest>,
	) -> Result<Response<InitiateBridgeTransferResponse>, Status> {
		// Extract the request data
		let req = request.into_inner();

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
}
