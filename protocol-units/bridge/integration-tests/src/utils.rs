#![allow(dead_code)]
use crate::{HarnessMvtClient, HarnessMvtClientFramework, MovementToEthCallArgs};
use alloy::hex;
use anyhow::Result;
use aptos_sdk::{
	coin_client::CoinClient, rest_client::Transaction, types::account_address::AccountAddress,
};
use bridge_service::chains::bridge_contracts::{BridgeContract, BridgeContractError};
use bridge_service::chains::movement::{client::MovementClient, client_framework::MovementClientFramework};
use bridge_service::chains::movement::utils::{
	self as movement_utils, MovementAddress, MovementHash,
};
use bridge_service::types::{Amount, AssetType, BridgeAddress, BridgeTransferDetails, HashLock};
use serde_json::Value;
use tracing::{debug, info};

const FRAMEWORK_ADDRESS: AccountAddress = AccountAddress::new([
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
]);

pub fn assert_bridge_transfer_details(
	details: &BridgeTransferDetails<MovementAddress>, // MovementAddress for initiator
	expected_bridge_transfer_id: [u8; 32],
	expected_hash_lock: [u8; 32],
	expected_sender_address: AccountAddress,
	expected_recipient_address: Vec<u8>,
	expected_amount: u64,
	expected_state: u8,
) {
	assert_eq!(details.bridge_transfer_id.0, expected_bridge_transfer_id);
	assert_eq!(details.hash_lock.0, expected_hash_lock);
	assert_eq!(details.initiator_address.0 .0, expected_sender_address);
	assert_eq!(details.recipient_address.0, expected_recipient_address);
	assert_eq!(details.amount.0, AssetType::Moveth(expected_amount));
	assert_eq!(details.state, expected_state, "Bridge transfer state mismatch.");
}

pub fn assert_counterparty_bridge_transfer_details_framework(
	details: &BridgeTransferDetails<MovementAddress>, 
	expected_sender_address: String,
	expected_recipient_address: Vec<u8>,
	expected_amount: u64,
	expected_hash_lock: [u8; 32],
	expected_time_lock: u64
) {

	
	assert_eq!(details.initiator_address.to_string(), expected_sender_address);
	assert_eq!(details.recipient_address, BridgeAddress(expected_recipient_address));
	assert_eq!(details.amount, Amount(AssetType::Moveth(expected_amount)));
	assert_eq!(details.hash_lock.0, expected_hash_lock);
	assert_eq!(details.time_lock.0, expected_time_lock);
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
					match struct_tag.name.as_str() {
						"BridgeTransferInitiatedEvent" | "BridgeTransferLockedEvent" => {
							if let Some(bridge_transfer_id) =
								event.data.get("bridge_transfer_id").and_then(|v| v.as_str())
							{
								let hex_str = bridge_transfer_id.trim_start_matches("0x");
								let decoded_vec = hex::decode(hex_str).map_err(|_| {
									anyhow::Error::msg("Failed to decode hex string into Vec<u8>")
								})?;
								return decoded_vec.try_into().map_err(|_| {
									anyhow::Error::msg(
										"Failed to convert decoded Vec<u8> to [u8; 32]",
									)
								});
							}
						}
						_ => {}
					}
				}
			}
		}
	}
	Err(anyhow::Error::msg("No matching transaction found"))
}

pub async fn extract_bridge_transfer_id_framework(
        movement_client: &mut MovementClientFramework,
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
                                        match struct_tag.name.as_str() {
                                                "BridgeTransferInitiatedEvent" | "BridgeTransferLockedEvent" => {
                                                        if let Some(bridge_transfer_id_str) =
                                                                event.data.get("bridge_transfer_id").and_then(|v| v.as_str())
                                                        {
                                                                let bridge_transfer_id_vec = hex::decode(bridge_transfer_id_str.trim_start_matches("0x"))
                                                                        .map_err(|_| anyhow::Error::msg("Failed to decode hex string into Vec<u8>"))?;
                                                                let bridge_transfer_id: [u8; 32] = bridge_transfer_id_vec.try_into()
                                                                        .map_err(|_| anyhow::Error::msg("Failed to convert Vec<u8> to [u8; 32]"))?;
                                                                
								return Ok(bridge_transfer_id);
                                                        }
                                                }
                                                _ => {}
                                        }
                                }
                        }
                }
        }
        Err(anyhow::Error::msg("No matching transaction found"))
}

pub async fn fetch_bridge_transfer_details(
        movement_client: &mut MovementClientFramework,
        bridge_transfer_id: Vec<u8>,
) -> Result<BridgeTransferDetails<AccountAddress>, anyhow::Error> {
        let rest_client = movement_client.rest_client();
        let account_address = FRAMEWORK_ADDRESS;
        let resource_tag = "0x1::atomic_bridge_store::SmartTableWrapper<vector<u8>, 0x1::atomic_bridge_store::BridgeTransferDetails<address, 0x1::ethereum::EthereumAddress>>";
        
        let resource_response = rest_client
            .get_account_resource(account_address, resource_tag)
            .await
            .map_err(|e| anyhow::Error::msg(format!("Failed to fetch resource: {:?}", e)))?;
        
        let json_value: Value = resource_response.into_inner().unwrap().data;
        
        if let Some(transfers) = json_value.get("inner").and_then(|t| t.get("buckets")) {
                for (key, value) in transfers.as_object().unwrap().iter() {
                        let key_vec = hex::decode(key).expect("Failed to decode key"); // Convert the key into Vec<u8>
                        
                        if key_vec == bridge_transfer_id {
                                let bridge_transfer_details: BridgeTransferDetails<AccountAddress> =
                                    serde_json::from_value(value.clone())
                                    .map_err(|e| anyhow::Error::msg(format!("Failed to deserialize BridgeTransferDetails: {:?}", e)))?;
                                return Ok(bridge_transfer_details);
                        }
                }
        }
        
        Err(anyhow::Error::msg("No matching bridge transfer details found"))
}

pub async fn fund_and_check_balance(
	movement_harness: &mut HarnessMvtClient,
	expected_balance: u64,
) -> Result<()> {
	let movement_client_signer = movement_harness.movement_client.signer();
	let rest_client = movement_harness.rest_client.clone();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_harness.faucet_client.write().unwrap();
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

pub async fn fund_and_check_balance_framework(
	movement_harness: &mut HarnessMvtClientFramework,
	expected_balance: u64,
) -> Result<()> {
	let movement_client_signer = movement_harness.movement_client.signer();
	let rest_client = movement_harness.rest_client.clone();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_harness.faucet_client.write().unwrap();
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

pub async fn initiate_bridge_transfer_helper(
	movement_client: &mut MovementClient,
	initiator_address: AccountAddress,
	recipient_address: Vec<u8>,
	hash_lock: [u8; 32],
	amount: u64,
	timelock_modify: bool,
) -> Result<(), BridgeContractError> {
	// Publish for test

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
			BridgeAddress(MovementAddress(initiator_address)),
			BridgeAddress(recipient_address),
			HashLock(MovementHash(hash_lock).0),
			Amount(AssetType::Moveth(amount)),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	Ok(())
}

pub async fn initiate_bridge_transfer_helper_framework(
	movement_client: &mut MovementClientFramework,
	initiator_address: AccountAddress,
	recipient_address: Vec<u8>,
	hash_lock: [u8; 32],
	amount: u64,
) -> Result<(), BridgeContractError> {
	movement_client
		.initiate_bridge_transfer(
			BridgeAddress(MovementAddress(initiator_address)),
			BridgeAddress(recipient_address),
			HashLock(MovementHash(hash_lock).0),
			Amount(AssetType::Moveth(amount)),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	Ok(())
}
