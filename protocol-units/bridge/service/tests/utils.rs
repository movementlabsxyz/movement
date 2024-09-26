use alloy::hex;
use anyhow::Result;
use aptos_sdk::{
	coin_client::CoinClient, rest_client::Transaction, types::account_address::AccountAddress,
};
use bridge_service::chains::bridge_contracts::{BridgeContract, BridgeContractError};
use bridge_service::types::{
	Amount, AssetType, BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress
};
use bridge_service::chains::movement::client::MovementClient;
use bridge_service::chains::movement::utils::{self as movement_utils, MovementHash, MovementAddress};
use tracing::debug;

pub fn assert_bridge_transfer_details(
	details: &BridgeTransferDetails<MovementAddress>, // MovementAddress for initiator
	expected_bridge_transfer_id: [u8; 32],
	expected_hash_lock: [u8; 32],
	expected_sender_address: AccountAddress,
	expected_recipient_address: Vec<u8>,
	expected_amount: u64,
	expected_state: u8,
) 
{
	assert_eq!(details.bridge_transfer_id.0, expected_bridge_transfer_id);
	assert_eq!(details.hash_lock.0, expected_hash_lock);
	assert_eq!(details.initiator_address.0 .0, expected_sender_address);
	assert_eq!(details.recipient_address.0, expected_recipient_address);
	assert_eq!(details.amount.0, AssetType::Moveth(expected_amount));
	assert_eq!(details.state, expected_state, "Bridge transfer state mismatch.");
}

pub async fn extract_bridge_transfer_id(
	movement_client: &mut MovementClient,
) -> Result<[u8; 32], anyhow::Error> {
	let sender_address = movement_client.signer().address();
	let sequence_number = 0; // Modify as needed
	let rest_client = movement_client.rest_client();

	let transactions = rest_client
		.get_account_transactions(sender_address, Some(sequence_number), Some(20))
		.await
		.map_err(|e| anyhow::Error::msg(format!("Failed to get transactions: {:?}", e)))?;

	if let Some(transaction) = transactions.into_inner().last() {
		if let Transaction::UserTransaction(user_txn) = transaction {
			for event in &user_txn.events {
				if let aptos_sdk::rest_client::aptos_api_types::MoveType::Struct(struct_tag) =
					&event.typ
				{
					if struct_tag.module.as_str() == "atomic_bridge_initiator"
						&& struct_tag.name.as_str() == "BridgeTransferInitiatedEvent"
					{
						if let Some(bridge_transfer_id) =
							event.data.get("bridge_transfer_id").and_then(|v| v.as_str())
						{
							let hex_str = bridge_transfer_id.trim_start_matches("0x");
							let decoded_vec = hex::decode(hex_str).map_err(|_| {
								anyhow::Error::msg("Failed to decode hex string into Vec<u8>")
							})?;
							return decoded_vec.try_into().map_err(|_| {
								anyhow::Error::msg("Failed to convert decoded Vec<u8> to [u8; 32]")
							});
						}
					}
				}
			}
		}
	}
	Err(anyhow::Error::msg("No matching transaction found"))
}

pub async fn fund_and_check_balance(
	movement_client: &mut MovementClient,
	expected_balance: u64,
) -> Result<()> {
	let movement_client_signer = movement_client.signer();
	let rest_client = movement_client.rest_client();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client
		.faucet_client()
		.expect("Failed to get FaucetClient")
		.write()
		.unwrap();
	faucet_client.fund(movement_client_signer.address(), expected_balance).await?;

	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	assert!(
		balance >= expected_balance,
		"Expected Movement Client to have at least {}, but found {}",
		expected_balance,
		balance
	);

	Ok(())
}

pub async fn publish_for_test(movement_client: &mut MovementClient) {
	let _ = movement_client.publish_for_test();
}

pub async fn initiate_bridge_transfer_helper(
	movement_client: &mut MovementClient,
	initiator_address: AccountAddress,
	recipient_address: Vec<u8>,
	hash_lock: [u8; 32],
	amount: u64,
	timelock_modify: bool,
) -> Result<(), BridgeContractError> {
	// Publish for test
	let _ = movement_client.publish_for_test();

	if timelock_modify {
		// Set the timelock to 1 second for testing
		movement_client.initiator_set_timelock(1).await.expect("Failed to set timelock");
	}

	// Mint MovETH to the initiator's address
	let mint_amount = 200 * 100_000_000; // Assuming 8 decimals for MovETH

	let mint_args = vec![
		movement_utils::serialize_address_initiator(&movement_client.signer().address())?, // Mint to initiator's address
		movement_utils::serialize_u64_initiator(&mint_amount)?, // Amount to mint (200 MovETH)
	];

	let mint_payload = movement_utils::make_aptos_payload(
		movement_client.native_address, // Address where moveth module is published
		"moveth",
		"mint",
		Vec::new(),
		mint_args,
	);

	// Send transaction to mint MovETH
	movement_utils::send_and_confirm_aptos_transaction(
		&movement_client.rest_client(),
		movement_client.signer(),
		mint_payload,
	)
	.await
	.map_err(|_| BridgeContractError::MintError)?;

	debug!("Successfully minted 200 MovETH to the initiator");

	// Initiate the bridge transfer
	movement_client
		.initiate_bridge_transfer(
			InitiatorAddress(MovementAddress(initiator_address)),
			RecipientAddress(recipient_address),
			HashLock(MovementHash(hash_lock).0),
			Amount(AssetType::Moveth(amount)),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	Ok(())
}
