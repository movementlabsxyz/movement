use alloy_primitives::keccak256;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::{MovementToEthCallArgs, TestHarness};
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use bridge_service::{
	chains::{
		movement::utils::MovementAddress,
		ethereum::types::EthAddress
	},
	types::{Amount, BridgeAddress, BridgeTransferId},
};
use bridge_util::types::Nonce;
use bridge_util::BridgeClientContract;
use bridge_util::BridgeContractEvent;
use bridge_util::BridgeRelayerContract;
use futures::StreamExt;
use tokio::{self};
use rand::Rng;

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = MovementToEthCallArgs::default();

	let test_result = async {
		mvt_client_harness
			.fund_signer_and_check_balance_framework(100_000_000_000)
			.await?;
		{
			tracing::info!("Before intiate_bridge_transfer");
			let res = BridgeClientContract::initiate_bridge_transfer(
				&mut mvt_client_harness.movement_client,
				BridgeAddress(args.recipient.clone()),
				Amount(args.amount),
			)
			.await?;

			tracing::info!("Initiate result: {:?}", res);
		}

		// Wait for the tx to be executed
		tracing::info!("Wait for the Movement Initiated event.");
		let (_, mvt_health_rx) = tokio::sync::mpsc::channel(10);
		let mut mvt_monitoring =
			MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

		// Use timeout to wait for the next event
		let event_option =
			tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
				.await
				.expect("Timeout while waiting for the Movement Initiated event");

		// Check if we received an event (Option) and handle the Result inside it
		let (
			bridge_transfer_id, 
			initiator,
			recipient,
			amount,
			nonce
		) = match event_option {
			Some(Ok(BridgeContractEvent::Initiated(detail))) => {
				(
					detail.bridge_transfer_id, 
					detail.initiator,
					detail.recipient,
					detail.amount,
					detail.nonce
				)
			}
			Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
			None => panic!("No event received"),
			_ => panic!("Not a an Initiated event: {:?}", event_option),
		};

		tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

		assert_eq!(initiator.0 .0, mvt_client_harness.signer_address());
		assert_eq!(recipient, BridgeAddress(args.recipient.clone()));
		assert_eq!(amount, Amount(args.amount));
		assert_eq!(nonce, Nonce(12));

		Ok(())
	}
	.await;

	test_result
}

fn hex_to_bytes(input: Vec<u8>) -> Vec<u8> {
	let mut result = Vec::new();
	assert!(input.len() % 2 == 0, "Input length must be even for valid hex");

	let mut i = 0;
	while i < input.len() {
		let high_nibble = ascii_hex_to_u8(input[i]);
		let low_nibble = ascii_hex_to_u8(input[i + 1]);
		let byte = (high_nibble << 4) | low_nibble;
		result.push(byte);
		i += 2;
	}

	result
}

fn ascii_hex_to_u8(ch: u8) -> u8 {
	match ch {
		b'0'..=b'9' => ch - b'0',
		b'A'..=b'F' => ch - b'A' + 10,
		b'a'..=b'f' => ch - b'a' + 10,
		_ => panic!("Invalid hex character: {}", ch),
	}
}

fn normalize_to_32_bytes(value: Vec<u8>) -> Vec<u8> {
	let mut meaningful = Vec::new();
	let mut i = 0;

	// Remove trailing zeroes
	while i < value.len() {
		if value[i] != 0 {
			meaningful.push(value[i]);
		}
		i += 1;
	}

	let mut result = Vec::with_capacity(32);
	let padding_length = 32 - meaningful.len();

	// Pad with zeros on the left
	for _ in 0..padding_length {
		result.push(0);
	}

	// Append the meaningful bytes
	result.extend_from_slice(&meaningful);

	result
}
#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
        let (mut mvt_client_harness, config) =
                TestHarness::new_with_movement().await.expect("Bridge config file not set");
        let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
        let mut mvt_monitoring =
                MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

        // Set initiator as hex string
        let initiator = hex::decode("32Be343B94f860124dC4fEe278FDCBD38C102D88")
                .expect("Failed to decode initiator hex");

        // Set recipient address
        let recipient = AccountAddress::new(*b"0x00000000000000000000000000fade");

        // Set amount to 1
        let amount = Amount(1);

        // Random nonce
        let incoming_nonce = TestHarness::create_nonce();

        // Combine bytes for hashing bridge_transfer_id
        let mut combined_bytes = Vec::new();
        combined_bytes.extend(&initiator);
        combined_bytes.extend(bcs::to_bytes(&recipient).expect("Failed to serialize recipient"));
        combined_bytes.extend(normalize_to_32_bytes(
                bcs::to_bytes(&amount).expect("Failed to serialize amount"),
        ));
        combined_bytes.extend(normalize_to_32_bytes(
                bcs::to_bytes(&incoming_nonce).expect("Failed to serialize nonce"),
        ));

        // Compute bridge_transfer_id using Keccak-256 hash
        let bridge_transfer_id = keccak256(combined_bytes);

        let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
        let movement_client_signer = mvt_client_harness.movement_client.signer();

        // Fund accounts
        {
                let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
                faucet_client
                        .fund(movement_client_signer.address(), 100_000_000)
                        .await?;
                faucet_client.fund(recipient, 100_000_000).await?;
        }

        // Assert the balance is sufficient
        let balance = coin_client
                .get_account_balance(&movement_client_signer.address())
                .await?;
        assert!(
                balance >= 100_000_000,
                "Expected Movement Client to have at least 100_000_000, but found {}",
                balance
        );

        // Call the complete_bridge_transfer function
        BridgeRelayerContract::complete_bridge_transfer(
                &mut mvt_client_harness.movement_client,
                BridgeTransferId(bridge_transfer_id.into()),
                BridgeAddress(initiator.clone()),
                BridgeAddress(MovementAddress(recipient)),
                amount,
                incoming_nonce,
        )
        .await
        .expect("Failed to complete bridge transfer");

        // Wait for the Movement-side Completed event
        tracing::info!("Wait for Movement-side Completed event.");
        loop {
                let event = tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
                        .await
                        .expect("Wait for completed event timeout.");
                if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
                        tracing::info!("Completed details: {:?}", detail);
                        assert_eq!(
                                detail.bridge_transfer_id,
                                BridgeTransferId(bridge_transfer_id.into()),
                                "Bad transfer id in completed event"
                        );
                        assert_eq!(detail.nonce, incoming_nonce, "Bad nonce in completed event");
                        assert_eq!(detail.amount, amount, "Bad amount in completed event");
                        assert_eq!(
                                detail.initiator.0,
                                initiator,
                                "Bad initiator address in completed event"
                        );
                        assert_eq!(
                                detail.recipient.0.0,
                                recipient,
                                "Bad recipient address in completed event"
                        );
                        break;
                }
        }

        Ok(())
}

