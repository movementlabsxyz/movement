use tokio::time::{sleep, Duration}; // Add these imports


use alloy::{
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

use rand;
use tiny_keccak::{Keccak, Hasher as KeccakHasher};
use tokio::{self, process::{Child, Command}};
use futures::{channel::mpsc::{self, UnboundedReceiver}, StreamExt};

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
        let _ = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .try_init();
	let (scaffold, mut child) = TestHarness::new_with_movement().await;
	let movement_client = scaffold.movement_client().expect("Failed to get MovementClient");
	//
	let rest_client = movement_client.rest_client();
        let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client.faucet_client().expect("Failed to get // FaucetClient");
	let movement_client_signer = movement_client.signer();

	let faucet_client = faucet_client.write().unwrap();

	faucet_client
	.fund(movement_client_signer.address(), 100_000_000)
	.await?;
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
        let _ = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .try_init();

        let (mut harness, mut child) = TestHarness::new_with_movement().await;

        let initiator_addr = AccountAddress::new(*b"0x123456789abcdef123456789abcdef");
        let recipient = b"0x123456789abcdef".to_vec();
        let hash_lock = *keccak256(b"secret".to_vec());
        let time_lock = 3600;
        let amount = 100;

        // Generate bridge_transfer_id in the test (same logic as Move function)
        let mut combined_bytes = vec![];
        combined_bytes.extend_from_slice(&bcs::to_bytes(&initiator_addr)?);
        combined_bytes.extend_from_slice(&recipient);
        combined_bytes.extend_from_slice(&hash_lock);
        let nonce = 1u64; // Assuming nonce is 1, can be fetched or managed
        combined_bytes.extend_from_slice(&bcs::to_bytes(&nonce)?);
        let mut hasher = Keccak::v256();
        // Feed the input data into the hasher
        hasher.update(&combined_bytes);
        // Create an array to store the result
        let mut bridge_transfer_id = [0u8; 32];
        // Finalize the hash and store it in the array
        hasher.finalize(&mut bridge_transfer_id);

        let test_result = async {
                let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
                let _ = movement_client.publish_for_test();

                let rest_client = movement_client.rest_client();
                let coin_client = CoinClient::new(&rest_client);
                let faucet_client = movement_client.faucet_client().expect("Failed to get FaucetClient");
                let movement_client_signer = movement_client.signer();

                // Fund account for testing
                {
                        let faucet_client = faucet_client.write().unwrap();
                        faucet_client.fund(movement_client_signer.address(), 100_000_000_000).await?;
                }

                // Verify account balance
                let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
                assert!(balance >= 100_000_000_000, "Expected Movement Client to have at least 100_000_000, but found {}", balance);

                // Initiate bridge transfer using the precomputed `bridge_transfer_id`
                movement_client
                        .initiate_bridge_transfer(
                                InitiatorAddress(MovementAddress(initiator_addr)),
                                RecipientAddress(recipient.clone()),
                                HashLock(hash_lock),
                                TimeLock(time_lock),
                                Amount(AssetType::Moveth(amount)),
                        ).await.expect("Failed to initiate bridge transfer");

                let details = BridgeContractInitiator::get_bridge_transfer_details(
                        movement_client,
                        BridgeTransferId(bridge_transfer_id)
                ).await
                .expect("Failed to get bridge transfer details")
                .expect("Expected to find bridge transfer details, but got None");

                assert_eq!(details.bridge_transfer_id.0, bridge_transfer_id);
                assert_eq!(details.hash_lock.0, hash_lock);
                assert_eq!(&details.initiator_address.0 .0[32 - recipient.len()..], &recipient);
                assert_eq!(details.recipient_address.0, recipient);
                assert_eq!(details.amount.0, AssetType::Moveth(amount));
                assert_eq!(details.state, 1, "Bridge transfer should be locked.");

                BridgeContractInitiator::complete_bridge_transfer(
                        movement_client,
                        BridgeTransferId(bridge_transfer_id),
                        HashLockPreImage(b"secret".to_vec())
                ).await
                .expect("Failed to complete bridge transfer");

                let details = BridgeContractInitiator::get_bridge_transfer_details(
                        movement_client,
                        BridgeTransferId(bridge_transfer_id)
                ).await
                .expect("Failed to get bridge transfer details")
                .expect("Expected to find bridge transfer details, but got None");

                assert_eq!(details.state, 2, "Bridge transfer should be completed.");
                Ok(())
        }.await;

        if let Err(e) = child.kill().await {
                eprintln!("Failed to kill child process: {:?}", e);
        }

        test_result
}


// Todo:
// test_movement_client_call_complete_bridge_transfer
// test_movement_client_call_refund_bridge_transfer
// test_movement_client_get_bridge_transfer_details
