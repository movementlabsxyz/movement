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
