use futures::StreamExt;
use test_log::test;

use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, Convert, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, TimeLock,
	},
};

use crate::shared::{
	setup_bridge_service, B1Client, B2Client, BC1Address, BC1Hash, BC2Address, BC2Hash,
	SetupBridgeServiceResult,
};

#[test(tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn test_bridge_service_error_handling() {
	let SetupBridgeServiceResult(
		mut bridge_service,
		mut blockchain_1_client,
		mut blockchain_2_client,
		blockchain_1,
		blockchain_2,
	) = setup_bridge_service();

	tokio::spawn(blockchain_1);
	tokio::spawn(blockchain_2);

	// Step 1: Initiating the swap on Blockchain 1 with an invalid hash lock

	tracing::debug!("Initiating bridge transfer with invalid hash lock");
	let result = blockchain_1_client
		.initiate_bridge_transfer(
			InitiatorAddress(BC1Address("initiator")),
			RecipientAddress(BC1Address("recipient")),
			HashLock(BC1Hash::from("invalid_hash_lock")),
			TimeLock(100),
			Amount(1000),
		)
		.await;

	assert!(result.is_err());
	tracing::debug!("Expected error: {:?}", result.err());

	// Step 2: Attempting to complete the swap on Blockchain 2 with an invalid secret

	tracing::debug!("Attempting to complete bridge transfer with invalid secret");
	let result = <B2Client as BridgeContractCounterparty>::complete_bridge_transfer(
		&mut blockchain_2_client,
		BridgeTransferId(BC2Hash::from("non_existent_transfer_id")),
		HashLockPreImage(b"invalid_secret".to_vec()),
	)
	.await;

	assert!(result.is_err());
	tracing::debug!("Expected error: {:?}", result.err());
}
