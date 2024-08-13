use alloy::{
	node_bindings::Anvil,
	primitives::{address, keccak256},
	providers::Provider,
};
use bridge_integration_tests::TestHarness;
use bridge_shared::{
	bridge_contracts::BridgeContractInitiator,
	types::{Amount, HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use ethereum_bridge::types::EthAddress;

use aptos_sdk::types::LocalAccount; 
use rand::{rngs::StdRng, SeedableRng}; 
use anyhow::Result; 
use tokio;
use aptos_logger::Logger;
use aptos_language_e2e_tests::{
	account::Account, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
    };
    use aptos_types::{
	account_config::{DepositEvent, WithdrawEvent},
	transaction::{ExecutionStatus, SignedTransaction, TransactionOutput, TransactionStatus},
    };
    use std::{convert::TryFrom, time::Instant};

#[tokio::test]
async fn test_movement_client_should_build_and_fetch_accounts() {
	let scaffold: TestHarness = TestHarness::new_with_movement().await;
        Logger::init_for_testing();
        let mut executor = FakeExecutor::from_head_genesis();
        // create and publish a sender and receiver
        let sender = executor.create_raw_account_data(1_000_000_000_000, 10);
        let receiver = executor.create_raw_account_data(1_000_000_000_000, 10);
        executor.add_account_data(&sender);
        executor.add_account_data(&receiver);

        let transfer_amount = 1_000;
        let txn = peer_to_peer_txn(sender.account(), receiver.account(), 10, transfer_amount, 10000);

        let output = executor.execute_transaction(txn);
        assert_eq!(
                output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
        );

        executor.apply_write_set(output.write_set());

        // check that numbers in stored DB are correct
        let sender_balance = 1_000_000_000_000 - transfer_amount;
        let receiver_balance = 1_000_000_000_000 + transfer_amount;
        let updated_sender = executor
                .read_account_resource(sender.account())
                .expect("sender must exist");
        let updated_sender_balance = executor
                .read_coin_store_resource(sender.account())
                .expect("sender balance must exist");
        let updated_receiver_balance = executor
                .read_coin_store_resource(receiver.account())
                .expect("receiver balance must exist");
        assert_eq!(receiver_balance, updated_receiver_balance.coin());
        //assert_eq!(sender_balance, updated_sender_balance.coin());
        assert_eq!(11, updated_sender.sequence_number());
        assert_eq!(0, updated_sender_balance.deposit_events().count(),);
        assert_eq!(1, updated_receiver_balance.deposit_events().count());

        let rec_ev_path = receiver.received_events_key();
        let sent_ev_path = sender.sent_events_key();
        for event in output.events() {
                let event_key = event.event_key();
                if let Some(event_key) = event_key {
                        assert!(rec_ev_path == event_key || sent_ev_path == event_key);
                }
        }
}

#[tokio::test]
async fn test_eth_client_should_build_and_fetch_accounts() {
	let scaffold: TestHarness = TestHarness::new_only_eth().await;

	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let _anvil = Anvil::new().port(eth_client.rpc_port()).spawn();

	let expected_accounts = vec![
		address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266"),
		address!("70997970c51812dc3a010c7d01b50e0d17dc79c8"),
		address!("3c44cdddb6a900fa2b585dd299e03d12fa4293bc"),
		address!("90f79bf6eb2c4f870365e785982e1f101e93b906"),
		address!("15d34aaf54267db7d7c367839aaf71a00a2c6a65"),
		address!("9965507d1a55bcc2695c58ba16fb37d819b0a4dc"),
		address!("976ea74026e726554db657fa54763abd0c3a0aa9"),
		address!("14dc79964da2c08b23698b3d3cc7ca32193d9955"),
		address!("23618e81e3f5cdf7f54c3d65f7fbc0abf5b21e8f"),
		address!("a0ee7a142d267c1f36714e4a8f75612f20a79720"),
	];

	let provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	println!("provider: {:?}", provider);
	let accounts = provider.get_accounts().await.expect("Failed to get accounts");
	assert_eq!(accounts.len(), expected_accounts.len());

	for (account, expected) in accounts.iter().zip(expected_accounts.iter()) {
		assert_eq!(account, expected);
	}
}

#[tokio::test]
async fn test_client_should_deploy_initiator_contract() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let _ = harness.set_eth_signer(anvil.keys()[0].clone());

	let initiator_address = harness.deploy_initiator_contract().await;
	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");

	assert_eq!(initiator_address, expected_address);
}

#[tokio::test]
async fn test_client_should_successfully_call_initialize() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let _ = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;
}

#[tokio::test]
async fn test_client_should_successfully_call_initiate_transfer() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());

	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();

	harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(1000), // Eth
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_client_should_successfully_get_bridge_transfer_id() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();

	harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(1000), // Eth
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call get details with the captured event
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_client_should_successfully_complete_transfer() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;

	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	let _ = harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(1000),
			Amount(42),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call complete with the id captured from the event
}
