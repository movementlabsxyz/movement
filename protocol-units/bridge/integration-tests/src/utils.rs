use anyhow::Result;
use aptos_sdk::{
    rest_client::Transaction,
    types::account_address::AccountAddress,
};
use movement_bridge::MovementClient;
use alloy::hex;

pub async fn extract_bridge_transfer_id(
    movement_client: &MovementClient,
    sender_address: AccountAddress,
    sequence_number: u64,
) -> Result<[u8; 32], anyhow::Error> {
    let rest_client = movement_client.rest_client();
    let transactions = rest_client
        .get_account_transactions(sender_address, Some(sequence_number), Some(20))
        .await
        .map_err(|e| anyhow::Error::msg(format!("Failed to get transactions: {:?}", e)))?;

    if let Some(transaction) = transactions.into_inner().last() {
        if let Transaction::UserTransaction(user_txn) = transaction {
            for event in &user_txn.events {
                if let aptos_sdk::rest_client::aptos_api_types::MoveType::Struct(struct_tag) = &event.typ {
                    if struct_tag.module.as_str() == "atomic_bridge_initiator"
                        && struct_tag.name.as_str() == "BridgeTransferInitiatedEvent"
                    {
                        if let Some(bridge_transfer_id) = event.data.get("bridge_transfer_id").and_then(|v| v.as_str()) {
                            let hex_str = bridge_transfer_id.trim_start_matches("0x");
                            let decoded_vec = hex::decode(hex_str)
                                .map_err(|_| anyhow::Error::msg("Failed to decode hex string into Vec<u8>"))?;
                            return decoded_vec
                                .try_into()
                                .map_err(|_| anyhow::Error::msg("Failed to convert decoded Vec<u8> to [u8; 32]"));
                        }
                    }
                }
            }
        }
    }
    Err(anyhow::Error::msg("No matching transaction found"))
}
