use tokio::time::{sleep, Duration}; // Add these imports

use alloy::{
	hex,
	node_bindings::Anvil,
	primitives::{address, keccak256},
	providers::Provider,
};
use anyhow::Result;

use aptos_sdk::{coin_client::CoinClient, types::LocalAccount};
use bridge_integration_tests::TestHarness;
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	types::{
		Amount, AssetType, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
		RecipientAddress, TimeLock,
	},
};

use ethereum_bridge::types::EthAddress;
use movement_bridge::utils::MovementAddress;

use futures::{
	channel::mpsc::{self, UnboundedReceiver},
	StreamExt,
};
use rand;
use tiny_keccak::{Hasher as KeccakHasher, Keccak};
use tokio::{
	self,
	process::{Child, Command},
};

use aptos_types::account_address::AccountAddress;
use tracing::{debug, info};
use tracing_subscriber;

struct ChildGuard {
	child: Child,
}

impl Drop for ChildGuard {
	fn drop(&mut self) {
		let _ = self.child.kill();
	}
}

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
		let sequence_number = 0; // Replace this with the correct sequence number

		let mut bridge_transfer_id: Option<String> = None;

		let rest_client = movement_client.rest_client();
		let transactions = rest_client
			.get_account_transactions(sender_address, Some(sequence_number), Some(20))
			.await
			.map_err(|e| anyhow::Error::msg(format!("Failed to get transactions: {:?}", e)))?;

		if let Some(transaction) = transactions.into_inner().last() {
			if let aptos_sdk::rest_client::Transaction::UserTransaction(user_txn) = transaction {
				for event in &user_txn.events {
					// Log the entire event details for debugging
					info!("Event: {:?}", event);

					if let aptos_sdk::rest_client::aptos_api_types::MoveType::Struct(struct_tag) =
						&event.typ
					{
						if struct_tag.module.as_str() == "atomic_bridge_initiator"
							&& struct_tag.name.as_str() == "BridgeTransferInitiatedEvent"
						{
							bridge_transfer_id = Some(
								event
									.data
									.get("bridge_transfer_id")
									.ok_or_else(|| {
										anyhow::Error::msg("bridge_transfer_id not found")
									})?
									.as_str()
									.ok_or_else(|| {
										anyhow::Error::msg("Invalid bridge_transfer_id format")
									})?
									.to_string(),
							);

							info!("Extracted bridge transfer id: {:?}", bridge_transfer_id);
							break; // Stop searching after we find the event
						}
					}
				}

				if bridge_transfer_id.is_none() {
					return Err(anyhow::Error::msg("No matching event found in the transaction"));
				}
			} else {
				return Err(anyhow::Error::msg("Not a user transaction"));
			}
		} else {
			return Err(anyhow::Error::msg(
				"No transaction found for the provided sequence number",
			));
		}
		// Unwrap the bridge_transfer_id to be used later
		let bridge_transfer_id = bridge_transfer_id.unwrap();

		let hex_str = bridge_transfer_id.trim_start_matches("0x");

		// Decode the hex string into a Vec<u8>
		let decoded_vec = hex::decode(hex_str)
			.map_err(|_| anyhow::Error::msg("Failed to decode hex string into Vec<u8>"))?;

		// Convert the Vec<u8> into a [u8; 32]
		let bridge_transfer_id: [u8; 32] = decoded_vec
			.try_into()
			.map_err(|_| anyhow::Error::msg("Failed to convert decoded Vec<u8> to [u8; 32]"))?;

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

// Todo:
// test_movement_client_call_complete_bridge_transfer
// test_movement_client_call_refund_bridge_transfer
// test_movement_client_get_bridge_transfer_details
