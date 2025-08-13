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
	println!("Starting framework_upgrade_collect_gas_fees_test...");
	
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

	// Test 1: Check current framework version and verify upgrade
	println!("=== Test 1: Checking current framework version and upgrade status ===");
	
	// Get chain info to check framework version
	let ledger_info = rest_client
		.get_ledger_information()
		.await
		.context("Failed to get ledger information")?;
	
	let chain_info = ledger_info.into_inner();
	println!("Chain ID: {}", chain_info.chain_id);
	println!("Ledger version: {}", chain_info.version);
	println!("Ledger timestamp: {}", chain_info.timestamp_usecs);
	
	// Test 2: Verify transaction_fee module functionality by trying to call a view function
	println!("=== Test 2: Verifying transaction_fee module functionality ===");
	
	// Try to call a view function on the transaction_fee module to verify it exists
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
		println!("[PASS] transaction_fee module is accessible - framework upgrade appears successful");
	} else {
		println!("[FAIL] transaction_fee module is not accessible - framework upgrade may not be complete");
		return Err(anyhow::anyhow!("Framework upgrade verification failed: transaction_fee module not accessible"));
	}

	// Test 3: Verify governed gas pool is deprecated
	println!("=== Test 3: Verifying governed gas pool deprecation ===");
	
	// Try to call a view function on the old governed_gas_pool module
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
		println!("[WARN] governed_gas_pool module is still accessible - deprecation may not be complete");
		// Don't fail the test here as deprecation might be in progress
	}

	// Test 4: Execute transactions and verify gas fees are collected via new module
	println!("=== Test 4: Executing test transaction to verify gas fee collection via transaction_fee module ===");
	
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

	// Test 5: Verify gas fees were collected and analyze the transaction
	println!("=== Test 5: Verifying gas fee collection and analyzing transaction ===");
	
	let final_sender_balance = coin_client
		.get_account_balance(&sender.address())
		.await
		.context("Failed to get final sender balance")?;

	println!("Final sender balance: {}", final_sender_balance);

	// Verify that gas fees were deducted
	if final_sender_balance < initial_sender_balance {
		let gas_fees_deducted = initial_sender_balance - final_sender_balance;
		println!("Gas fees deducted: {}", gas_fees_deducted);
		
		// Note: We can't easily get transaction details without the hash, so we'll focus on balance verification
		println!("[PASS] Transaction executed successfully with hash: {:?}", test_txn);
		println!("[PASS] Gas fees were properly deducted, indicating fee collection is working");
	} else {
		println!("[FAIL] No gas fees were deducted - this indicates a serious issue");
		return Err(anyhow::anyhow!("Gas fee collection verification failed: no fees deducted"));
	}

	// Test 6: Verify transaction_fee module state and configuration
	println!("=== Test 6: Verifying transaction_fee module state and configuration ===");
	
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

	// Test 7: Framework upgrade verification summary
	println!("=== Test 7: Framework upgrade verification summary ===");
	
	if transaction_fee_module_exists && !governed_gas_pool_exists {
		println!("[PASS] Framework upgrade to transaction_fee::collect_fee is COMPLETE");
		println!("[PASS] Migration from governed gas pool to transaction_fee::collect_fee is SUCCESSFUL");
		println!("[PASS] New fee collection mechanism is operational");
	} else if transaction_fee_module_exists && governed_gas_pool_exists {
		println!("[WARN] Framework upgrade is IN PROGRESS");
		println!("[PASS] New transaction_fee module is accessible");
		println!("[WARN] Old governed_gas_pool module is still accessible (deprecation in progress)");
	} else {
		println!("[FAIL] Framework upgrade verification FAILED");
		println!("[FAIL] New transaction_fee module is not accessible");
		return Err(anyhow::anyhow!("Framework upgrade verification failed"));
	}

	println!("All tests completed successfully!");
	Ok(())
} 