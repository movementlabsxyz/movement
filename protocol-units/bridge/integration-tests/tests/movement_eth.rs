use tokio::time::{sleep, Duration}; // Add these imports

use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_integration_tests::{utils::{self as test_utils}, MovementToEthCallArgs};
use aptos_sdk::coin_client::CoinClient;
use bridge_integration_tests::TestHarness;
use bridge_shared::{
	bridge_contracts::{BridgeContractInitiator, BridgeContractInitiatorError},
	types::{
		Amount, AssetType, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
		RecipientAddress, TimeLock,
	},
};

use movement_bridge::utils::{self as movement_utils, MovementAddress};

use tokio::{
	self
};

use aptos_types::account_address::AccountAddress;
use tracing::{debug, info};
use tracing_subscriber;

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (scaffold, mut child) = TestHarness::new_with_movement().await;
	let movement_client = scaffold.movement_client().expect("Failed to get MovementClient");
	//
	let rest_client = movement_client.rest_client();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client.faucet_client().expect("Failed to get // FaucetClient");
	let movement_client_signer = movement_client.signer();

	let faucet_client = faucet_client.write().unwrap();

	faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	info!("Balance: {:?}", balance);
	assert!(
		balance >= 100_000_000,
		"Expected Movement Client to have at least 100_000_000, but found {}",
		balance
	);

	child.kill().await?;

	Ok(())
}

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let mut movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut movement_client,100_000_000_000).await?;

		test_utils::initiate_bridge_transfer_helper(
			&mut movement_client,
			args.initiator.0,         
			args.recipient.clone(),   
			args.hash_lock,           
			args.time_lock,           
			args.amount,             
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id(&mut movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");
		
		assert_eq!(details.bridge_transfer_id.0, bridge_transfer_id);
		assert_eq!(details.hash_lock.0, args.hash_lock);
		assert_eq!(details.initiator_address.0.0, sender_address);
		assert_eq!(details.recipient_address.0, args.recipient);
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 1, "Bridge transfer should be locked.");
	
		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_initiate_and_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let initiator_addr = AccountAddress::new(*b"0x123456789abcdef123456789abcdef");
	let recipient = b"0x123456789abcdef".to_vec();
        let preimage = "secret".to_string();
        let serialized_preimage = bcs::to_bytes(&preimage).unwrap();  // BCS serialization
        let hash_lock = *keccak256(&serialized_preimage);
	let time_lock = 3600;
	let amount = 100;

	let test_result = async {
		// Immutable borrow of movement_client to get the signer and perform funding
		{
			let movement_client =
				harness.movement_client_mut().expect("Failed to get MovementClient");
			let movement_client_signer = movement_client.signer();
			let rest_client = movement_client.rest_client();
			let coin_client = CoinClient::new(&rest_client);
			let faucet_client = movement_client
				.faucet_client()
				.expect("Failed to get FaucetClient")
				.write()
				.unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000_000).await?;

			let balance =
				coin_client.get_account_balance(&movement_client_signer.address()).await?;
			assert!(
				balance >= 100_000_000_000,
				"Expected Movement Client to have at least 100_000_000, but found {}",
				balance
			);
		} // End of immutable borrow scope

		// Mutable borrow to initiate the bridge transfer
		{
			let movement_client =
				harness.movement_client_mut().expect("Failed to get MovementClient");
			let _ = movement_client.publish_for_test();

                        let mint_amount = 200 * 100_000_000; // Assuming 8 decimals for MovETH

                        let mint_args = vec![
                                movement_utils::serialize_address_initiator(&movement_client.signer().address())?, // Mint to initiator's address
                                movement_utils::serialize_u64_initiator(&mint_amount)?,                     // Amount to mint (200 MovETH)
                        ];
         
                        let mint_payload = movement_utils::make_aptos_payload(
                                movement_client.counterparty_address, // Address where moveth module is published
                                "moveth",
                                "mint",
                                Vec::new(),
                                mint_args,
                        );
        
                        movement_utils::send_and_confirm_aptos_transaction(&movement_client.rest_client, movement_client.signer(), mint_payload)
                                .await
                                .map_err(|_| BridgeContractInitiatorError::MintError)?; // New error variant for mint failure
        
                        debug!("Successfully minted 200 MovETH to the initiator");

			movement_client
				.initiate_bridge_transfer(
					InitiatorAddress(MovementAddress(initiator_addr)),
					RecipientAddress(recipient.clone()),
					HashLock(hash_lock),
					TimeLock(time_lock),
					Amount(AssetType::Moveth(amount)),
				)
				.await
				.expect("Failed to initiate bridge transfer");
		} // End of mutable borrow scope

		// Immutable borrow to extract the bridge transfer ID from the transaction
		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id(movement_client).await?;

		info!("Bridge transfer id: {:?}", bridge_transfer_id);

		// Now get the transfer details
		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id.0, bridge_transfer_id);
		assert_eq!(details.hash_lock.0, hash_lock);
		assert_eq!(details.initiator_address.0.0, sender_address);
		assert_eq!(details.recipient_address.0, recipient);
		assert_eq!(details.amount.0, AssetType::Moveth(amount));
		assert_eq!(details.state, 1, "Bridge transfer should be locked.");

		// Complete the transfer
		BridgeContractInitiator::complete_bridge_transfer(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
			HashLockPreImage(b"secret".to_vec()),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 2, "Bridge transfer should be completed.");
		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_initiate_and_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let initiator_addr = AccountAddress::new(*b"0x123456789abcdef123456789abcdef");
	let recipient = b"0x123456789abcdef".to_vec();
        let preimage = "secret".to_string();
        let serialized_preimage = bcs::to_bytes(&preimage).unwrap();  // BCS serialization
        let hash_lock = *keccak256(&serialized_preimage);
	let time_lock = 1;
	let amount = 100;

	let test_result = async {
		// Immutable borrow of movement_client to get the signer and perform funding
		{
			let movement_client =
				harness.movement_client_mut().expect("Failed to get MovementClient");
			let movement_client_signer = movement_client.signer();
			let rest_client = movement_client.rest_client();
			let coin_client = CoinClient::new(&rest_client);
			let faucet_client = movement_client
				.faucet_client()
				.expect("Failed to get FaucetClient")
				.write()
				.unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000_000).await?;

			let balance =
				coin_client.get_account_balance(&movement_client_signer.address()).await?;
			assert!(
				balance >= 100_000_000_000,
				"Expected Movement Client to have at least 100_000_000, but found {}",
				balance
			);
		} // End of immutable borrow scope

		// Mutable borrow to initiate the bridge transfer
		{
			let movement_client =
				harness.movement_client_mut().expect("Failed to get MovementClient");
			let _ = movement_client.publish_for_test();

                        let mint_amount = 200 * 100_000_000; // Assuming 8 decimals for MovETH

                        let mint_args = vec![
                                movement_utils::serialize_address_initiator(&movement_client.signer().address())?, // Mint to initiator's address
                                movement_utils::serialize_u64_initiator(&mint_amount)?,                     // Amount to mint (200 MovETH)
                        ];
         
                        let mint_payload = movement_utils::make_aptos_payload(
                                movement_client.counterparty_address, // Address where moveth module is published
                                "moveth",
                                "mint",
                                Vec::new(),
                                mint_args,
                        );
        
                        movement_utils::send_and_confirm_aptos_transaction(&movement_client.rest_client, movement_client.signer(), mint_payload)
                                .await
                                .map_err(|_| BridgeContractInitiatorError::MintError)?; // New error variant for mint failure
        
                        debug!("Successfully minted 200 MovETH to the initiator");

			movement_client
				.initiate_bridge_transfer(
					InitiatorAddress(MovementAddress(initiator_addr)),
					RecipientAddress(recipient.clone()),
					HashLock(hash_lock),
					TimeLock(time_lock),
					Amount(AssetType::Moveth(amount)),
				)
				.await
				.expect("Failed to initiate bridge transfer");
		} // End of mutable borrow scope

		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();
		let sequence_number = 0; // Replace this with the correct sequence number

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id(movement_client).await?;

		info!("Bridge transfer id: {:?}", bridge_transfer_id);

		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id.0, bridge_transfer_id);
		assert_eq!(details.hash_lock.0, hash_lock);
		assert_eq!(details.initiator_address.0.0, sender_address);
		assert_eq!(details.recipient_address.0, recipient);
		assert_eq!(details.amount.0, AssetType::Moveth(amount));
		assert_eq!(details.state, 1, "Bridge transfer should be locked.");

                sleep(Duration::from_secs(2)).await;

		// Complete the transfer
		BridgeContractInitiator::refund_bridge_transfer(
			movement_client,
			BridgeTransferId(bridge_transfer_id)
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 3, "Bridge transfer should be refunded.");
		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}