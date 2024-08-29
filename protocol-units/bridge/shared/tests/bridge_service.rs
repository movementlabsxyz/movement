use std::time::Duration;

use futures::StreamExt;
use test_log::test;

use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	bridge_service::{active_swap::ActiveSwapConfig, BridgeServiceConfig},
	types::{
		Amount, BridgeTransferDetails, Convert, CounterpartyCompletedDetails, HashLock,
		HashLockPreImage, InitiatorAddress, LockDetails, RecipientAddress, TimeLock, AssetType
	},
};

use crate::shared::{
	setup_bridge_service, B1Client, B2Client, BC1Address, BC1Hash, BC2Address, BC2Hash,
	SetupBridgeServiceResult,
};

mod shared;

#[test(tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn test_bridge_service_integration_a_to_b() {
	let SetupBridgeServiceResult(
		mut bridge_service,
		mut blockchain_1_client,
		mut blockchain_2_client,
		blockchain_1,
		blockchain_2,
	) = setup_bridge_service(BridgeServiceConfig {
		active_swap: ActiveSwapConfig {
			error_attempts: 3,
			error_delay: Duration::from_secs(1),
			contract_call_timeout: Duration::from_secs(5),
		},
	});

	tokio::spawn(blockchain_1);
	tokio::spawn(blockchain_2);

	// Step 1: Initiating the swap on Blockchain 1

	// The initiator of the swap triggers a bridge transfer, simultaneously time-locking the assets
	// in the smart contract.
	tracing::debug!("Initiating bridge transfer");
	blockchain_1_client
		.initiate_bridge_transfer(
			InitiatorAddress(BC1Address("initiator")),
			RecipientAddress::from(BC1Address("recipient")),
			HashLock(BC1Hash::from("hash_lock")),
			TimeLock(100),
			Amount(AssetType::EthAndWeth((0,1000))),
		)
		.await
		.expect("initiate_bridge_transfer failed");

	// We expect the bridge to recognize the contract event and emit the appropriate message
	let transfer_initiated_event = bridge_service.next().await.expect("No event");
	let transfer_initiated_event =
		transfer_initiated_event.B1I_ContractEvent().expect("Not a B1I event");
	tracing::debug!(?transfer_initiated_event);
	assert_eq!(
		transfer_initiated_event,
		&BridgeContractInitiatorEvent::Initiated(BridgeTransferDetails {
			bridge_transfer_id: transfer_initiated_event.bridge_transfer_id().clone(),
			initiator_address: InitiatorAddress(BC1Address("initiator")),
			recipient_address: RecipientAddress::from(BC1Address("recipient")),
			hash_lock: HashLock(BC1Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			amount: Amount(AssetType::EthAndWeth((0,1000)))
		})
	);

	// Step 2: Locking the assets on the Blockchain 2

	// Upon recognizing the event, our bridge server has invoked the counterparty
	// contract on blockchain 2 to initiate asset locking within the smart contract.
	tracing::debug!("Locking assets on Blockchain 2");

	let counterparty_locked_event = bridge_service.next().await.expect("No event");
	let counterparty_locked_event =
		counterparty_locked_event.B2C_ContractEvent().expect("Not a B2C event");
	tracing::debug!(?counterparty_locked_event);
	assert_eq!(
		counterparty_locked_event,
		&BridgeContractCounterpartyEvent::Locked(LockDetails {
			bridge_transfer_id: Convert::convert(transfer_initiated_event.bridge_transfer_id()),
			hash_lock: HashLock(BC2Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			initiator_address: InitiatorAddress::from(BC1Address("initiator")),
			recipient_address: RecipientAddress(BC2Address("recipient")),
			amount: Amount(AssetType::EthAndWeth((0,1000))),
		})
	);

	// Step 3: Client completes the swap on Blockchain 2, revealing the pre_image of the hash lock

	// Once the assets are secured within the counterparty smart contract, the initiator is able
	// to execute the complete bridge transfer by disclosing the secret key required to unlock the assets.
	tracing::debug!("Client completing bridge transfer on Blockchain 2");

	<B2Client as BridgeContractCounterparty>::complete_bridge_transfer(
		&mut blockchain_2_client,
		Convert::convert(transfer_initiated_event.bridge_transfer_id()),
		HashLockPreImage(b"hash_lock".to_vec()),
	)
	.await
	.expect("complete_bridge_transfer failed");

	// As the claim was made by the counterparty, we anticipate the bridge to generate a bridge
	// contract counterpart event.
	let completed_event_counterparty = bridge_service.next().await.expect("No event");
	let completed_event_counterparty =
		completed_event_counterparty.B2C_ContractEvent().expect("Not a B2C event");
	tracing::debug!(?completed_event_counterparty);
	assert_eq!(
		completed_event_counterparty,
		&BridgeContractCounterpartyEvent::Completed(CounterpartyCompletedDetails {
			bridge_transfer_id: Convert::convert(transfer_initiated_event.bridge_transfer_id()),
			initiator_address: InitiatorAddress::from(BC1Address("initiator")),
			recipient_address: RecipientAddress(BC2Address("recipient")),
			hash_lock: HashLock(BC2Hash::from("hash_lock")),
			secret: HashLockPreImage(b"hash_lock".to_vec()),
			amount: Amount(AssetType::EthAndWeth((0,1000))),
		})
	);

	// Step 4: Bridge service completes the swap, using the secret to claim the funds on Blockchain 1

	// As the initiator has successfully claimed the funds on the Counterparty blockchain, the bridge
	// is now expected to finalize the swap by completing the necessary tasks on the initiator
	// blockchain.
	tracing::debug!("Bridge service completing bridge transfer on Blockchain 1");

	let completed_event_initiator = bridge_service.next().await.expect("No event");
	let completed_event_initiator =
		completed_event_initiator.B1I_ContractEvent().expect("Not a B1I event");
	tracing::debug!(?completed_event_initiator);
	assert_eq!(
		completed_event_initiator,
		&BridgeContractInitiatorEvent::Completed(
			transfer_initiated_event.bridge_transfer_id().clone()
		)
	);
}

#[test(tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn test_bridge_service_integration_b_to_a() {
	let SetupBridgeServiceResult(
		mut bridge_service,
		mut blockchain_1_client,
		mut blockchain_2_client,
		blockchain_1,
		blockchain_2,
	) = setup_bridge_service(BridgeServiceConfig {
		active_swap: ActiveSwapConfig {
			error_attempts: 3,
			error_delay: Duration::from_secs(1),
			contract_call_timeout: Duration::from_secs(5),
		},
	});

	tokio::spawn(blockchain_1);
	tokio::spawn(blockchain_2);

	// Step 1: Initiating the swap on Blockchain 2

	// The initiator of the swap triggers a bridge transfer, simultaneously time-locking the assets
	// in the smart contract.
	tracing::debug!("Initiating bridge transfer on Blockchain 2");
	blockchain_2_client
		.initiate_bridge_transfer(
			InitiatorAddress(BC2Address("initiator")),
			RecipientAddress::from(BC2Address("recipient")),
			HashLock(BC2Hash::from("hash_lock")),
			TimeLock(100),
			Amount(AssetType::EthAndWeth((0,1000))),
		)
		.await
		.expect("initiate_bridge_transfer failed");

	// We expect the bridge to recognize the contract event and emit the appropriate message
	let transfer_initiated_event = bridge_service.next().await.expect("No event");
	let transfer_initiated_event =
		transfer_initiated_event.B2I_ContractEvent().expect("Not a B2I event");
	tracing::debug!(?transfer_initiated_event);
	assert_eq!(
		transfer_initiated_event,
		&BridgeContractInitiatorEvent::Initiated(BridgeTransferDetails {
			bridge_transfer_id: transfer_initiated_event.bridge_transfer_id().clone(),
			initiator_address: InitiatorAddress(BC2Address("initiator")),
			recipient_address: RecipientAddress::from(BC2Address("recipient")),
			hash_lock: HashLock(BC2Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			amount: Amount(AssetType::EthAndWeth((0,1000)))
		})
	);

	// Step 2: Locking the assets on the Blockchain 1

	// Upon recognizing the event, our bridge server has invoked the counterparty
	// contract on blockchain 1 to initiate asset locking within the smart contract.
	tracing::debug!("Locking assets on Blockchain 1");

	let counterparty_locked_event = bridge_service.next().await.expect("No event");
	let counterparty_locked_event =
		counterparty_locked_event.B1C_ContractEvent().expect("Not a B1C event");
	tracing::debug!(?counterparty_locked_event);
	assert_eq!(
		counterparty_locked_event,
		&BridgeContractCounterpartyEvent::Locked(LockDetails {
			bridge_transfer_id: Convert::convert(transfer_initiated_event.bridge_transfer_id()),
			hash_lock: HashLock(BC1Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			initiator_address: InitiatorAddress::from(BC1Address("initiator")),
			recipient_address: RecipientAddress(BC1Address("recipient")),
			amount: Amount(AssetType::EthAndWeth((0,1000))),
		})
	);

	// Step 3: Client completes the swap on Blockchain 1, revealing the pre_image of the hash lock

	// Once the assets are secured within the counterparty smart contract, the initiator is able
	// to execute the complete bridge transfer by disclosing the secret key required to unlock the assets.
	tracing::debug!("Client completing bridge transfer on Blockchain 1");

	<B1Client as BridgeContractCounterparty>::complete_bridge_transfer(
		&mut blockchain_1_client,
		Convert::convert(transfer_initiated_event.bridge_transfer_id()),
		HashLockPreImage(b"hash_lock".to_vec()),
	)
	.await
	.expect("complete_bridge_transfer failed");

	// As the claim was made by the counterparty, we anticipate the bridge to generate a bridge
	// contract counterpart event.
	let completed_event_counterparty = bridge_service.next().await.expect("No event");
	let completed_event_counterparty =
		completed_event_counterparty.B1C_ContractEvent().expect("Not a B1C event");
	tracing::debug!(?completed_event_counterparty);
	assert_eq!(
		completed_event_counterparty,
		&BridgeContractCounterpartyEvent::Completed(CounterpartyCompletedDetails {
			bridge_transfer_id: Convert::convert(transfer_initiated_event.bridge_transfer_id()),
			initiator_address: InitiatorAddress::from(BC1Address("initiator")),
			recipient_address: RecipientAddress(BC1Address("recipient")),
			hash_lock: HashLock(BC1Hash::from("hash_lock")),
			secret: HashLockPreImage(b"hash_lock".to_vec()),
			amount: Amount(AssetType::EthAndWeth((0,1000))),
		})
	);

	// Step 4: Bridge service completes the swap, using the secret to claim the funds on Blockchain 2

	// As the initiator has successfully claimed the funds on the Counterparty blockchain, the bridge
	// is now expected to finalize the swap by completing the necessary tasks on the initiator
	// blockchain.
	tracing::debug!("Bridge service completing bridge transfer on Blockchain 2");

	let completed_event_initiator = bridge_service.next().await.expect("No event");
	let completed_event_initiator =
		completed_event_initiator.B2I_ContractEvent().expect("Not a B2I event");
	tracing::debug!(?completed_event_initiator);
	assert_eq!(
		completed_event_initiator,
		&BridgeContractInitiatorEvent::Completed(
			transfer_initiated_event.bridge_transfer_id().clone()
		)
	);
}
