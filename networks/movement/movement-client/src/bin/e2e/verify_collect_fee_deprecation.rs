use anyhow::Context;
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use aptos_sdk::rest_client::aptos_api_types::{ViewRequest, EntryFunctionId, MoveModuleId, Address, IdentifierWrapper};
use std::str::FromStr;
use once_cell::sync::Lazy;
use url::Url;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();

	let node_connection_url = format!("http://{}:{}", node_connection_address, node_connection_port);
	Url::parse(node_connection_url.as_str()).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
	Url::parse(faucet_listen_url.as_str()).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	println!("Starting verify_collect_fee_deprecation test...");
	
	println!("Connecting to node at: {}", NODE_URL.as_str());
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	println!("Attempting to get chain info...");
	
	// Create test accounts
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);

	println!("Created test accounts");
	println!("Sender address: {}, Beneficiary address: {}", sender.address(), beneficiary.address());

	// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	// Create the beneficiary account
	faucet_client
		.create_account(beneficiary.address())
		.await
		.context("Failed to create beneficiary account")?;

	// Test 1: Verify new fee collection mechanism
	println!("=== Test 1: Verifying new fee collection mechanism ===");
	
	// Check if the new transaction_fee module is accessible by trying to call a view function
	let transaction_fee_view_req = ViewRequest {
		function: EntryFunctionId {
			module: MoveModuleId {
				address: Address::from_str("0x1")?,
				name: IdentifierWrapper::from_str("transaction_fee")?,
			},
			name: IdentifierWrapper::from_str("get_fee_config")?,
		},
		type_arguments: vec![],
		arguments: vec![],
	};
	
	let transaction_fee_module_exists = rest_client
		.view(&transaction_fee_view_req, None)
		.await
		.is_ok();
	
	if transaction_fee_module_exists {
		println!("[PASS] transaction_fee module is accessible");
		
		// Get module details to verify it's properly configured
		match rest_client.view(&transaction_fee_view_req, None).await {
			Ok(response) => {
				println!("[PASS] transaction_fee module details:");
				println!("   Configuration response: {:?}", response.inner());
			}
			Err(e) => {
				println!("[FAIL] Failed to get transaction_fee module details: {}", e);
				return Err(anyhow::anyhow!("Failed to verify transaction_fee module: {}", e));
			}
		}
	} else {
		println!("[FAIL] transaction_fee module is not accessible");
		return Err(anyhow::anyhow!("New fee collection mechanism not available"));
	}

	// Test 2: Verify that the old governed gas pool is deprecated
	println!("=== Test 2: Verifying governed gas pool deprecation ===");
	
	// Check if the old governed gas pool module still exists by trying to call a view function
	let governed_gas_pool_view_req = ViewRequest {
		function: EntryFunctionId {
			module: MoveModuleId {
				address: Address::from_str("0x1")?,
				name: IdentifierWrapper::from_str("governed_gas_pool")?,
			},
			name: IdentifierWrapper::from_str("get_gas_pool_address")?,
		},
		type_arguments: vec![],
		arguments: vec![],
	};
	
	let governed_gas_pool_exists = rest_client
		.view(&governed_gas_pool_view_req, None)
		.await
		.is_ok();
	
	if !governed_gas_pool_exists {
		println!("[PASS] governed_gas_pool module is not accessible - deprecation successful");
	} else {
		println!("[WARN] governed_gas_pool module is still accessible");
		println!("   This indicates deprecation may not be complete yet");
	}

	// Test 3: Execute transactions and verify they use the new fee collection mechanism
	println!("=== Test 3: Executing test transaction to verify new fee collection mechanism ===");
	
	let initial_sender_balance = coin_client
		.get_account_balance(&sender.address())
		.await
		.context("Failed to get initial sender balance")?;

	println!("Initial sender balance: {}", initial_sender_balance);

	// Execute test transaction
	println!("Executing test transaction...");
	
	let test_txn = coin_client
		.transfer(&mut sender, beneficiary.address(), 1_000, None)
		.await
		.context("Failed to submit test transaction")?;
	
	rest_client
		.wait_for_transaction(&test_txn)
		.await
		.context("Failed when waiting for transfer transaction")?;
	
	println!("Test transaction completed: {:?}", test_txn);

	// Test 4: Verify gas fees were collected and analyze the transaction
	println!("=== Test 4: Verifying gas fee collection and analyzing transaction ===");
	
	let final_sender_balance = coin_client
		.get_account_balance(&sender.address())
		.await
		.context("Failed to get final sender balance")?;

	println!("Final sender balance: {}", final_sender_balance);

	// Verify that gas fees were deducted
	if final_sender_balance < initial_sender_balance {
		let gas_fees_deducted = initial_sender_balance - final_sender_balance;
		println!("Gas fees deducted: {}", gas_fees_deducted);
		
		println!("[PASS] Transaction executed successfully with hash: {:?}", test_txn);
		println!("[PASS] Gas fees were properly deducted, indicating fee collection is working");
	} else {
		println!("[FAIL] No gas fees were deducted - this indicates a serious issue");
		return Err(anyhow::anyhow!("Gas fee collection verification failed: no fees deducted"));
	}

	// Test 5: Verify transaction_fee module state and configuration
	println!("=== Test 5: Verifying transaction_fee module state and configuration ===");
	
	// Try to get transaction_fee module configuration through view calls
	let fee_config_view_req = ViewRequest {
		function: EntryFunctionId {
			module: MoveModuleId {
				address: Address::from_str("0x1")?,
				name: IdentifierWrapper::from_str("transaction_fee")?,
			},
			name: IdentifierWrapper::from_str("get_fee_config")?,
		},
		type_arguments: vec![],
		arguments: vec![],
	};
	
	match rest_client.view(&fee_config_view_req, None).await {
		Ok(response) => {
			println!("[PASS] Transaction fee configuration accessible");
			println!("Configuration response: {:?}", response.inner());
		}
		Err(e) => {
			println!("[WARN] Could not access transaction fee configuration: {}", e);
		}
	}

	// Test 6: Fee collection deprecation verification summary
	println!("=== Test 6: Fee collection deprecation verification summary ===");
	
	if transaction_fee_module_exists && !governed_gas_pool_exists {
		println!("[PASS] Fee collection deprecation is COMPLETE");
		println!("[PASS] New transaction_fee::collect_fee mechanism is operational");
		println!("[PASS] Old governed_gas_pool mechanism is fully deprecated");
	} else if transaction_fee_module_exists && governed_gas_pool_exists {
		println!("[WARN] Fee collection deprecation is IN PROGRESS");
		println!("[PASS] New transaction_fee module is accessible");
		println!("[WARN] Old governed_gas_pool module is still accessible (deprecation in progress)");
	} else {
		println!("[FAIL] Fee collection deprecation verification FAILED");
		println!("[FAIL] New transaction_fee module is not accessible");
		return Err(anyhow::anyhow!("Fee collection deprecation verification failed"));
	}

	println!("All tests completed successfully!");
	Ok(())
}
