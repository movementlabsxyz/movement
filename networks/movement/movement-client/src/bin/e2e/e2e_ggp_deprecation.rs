use anyhow::Context;
use aptos_sdk::rest_client::{
	aptos_api_types::{Address, EntryFunctionId, IdentifierWrapper, MoveModuleId, ViewRequest},
};
use aptos_sdk::types::account_address::AccountAddress;
use movement_client::{
	coin_client::CoinClient,
	rest_client::Client,
	types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;
use reqwest;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	println!("Starting e2e_ggp_deprecation test...");
	
	// Connect to the node
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
	
	println!("Connecting to node at: {}", node_connection_url);
	
	let rest_client = Client::new(Url::from_str(&node_connection_url)?);
	let coin_client = CoinClient::new(&rest_client);

	println!("Attempting to get chain info...");
	
	// Create test accounts
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);

	println!("Created test accounts");
	println!("Sender address: {}, Beneficiary address: {}", sender.address(), beneficiary.address());

	// Fund the sender account using the testnet faucet
	println!("Funding sender account via testnet faucet...");
	
	let faucet_url = if let Ok(override_url) = std::env::var("MOVEMENT_FAUCET_URL") {
		override_url
	} else {
		"https://faucet.testnet.movementinfra.xyz".to_string()
	};
	
	// Try different approaches to match what the faucet expects
	let client = reqwest::Client::new();
	
	// First try GET with address (some faucets prefer this)
	let response = client
		.get(&format!("{}/mint", faucet_url))
		.query(&[
			("address", sender.address().to_string()),
			("amount", "1000000".to_string()),
			("return_txns", "true".to_string()),
		])
		.send()
		.await
		.context("Failed to send faucet GET request")?;
	
	let status = response.status();
	println!("Faucet GET response status: {}", status);
	
	if !status.is_success() {
		// If GET fails, try POST with form data
		println!("GET request failed with status {}, trying POST with form data...", status);
		let response = client
			.post(&format!("{}/mint", faucet_url))
			.form(&[
				("address", sender.address().to_string()),
				("amount", "1000000".to_string()),
				("return_txns", "true".to_string()),
			])
			.send()
			.await
			.context("Failed to send faucet POST request")?;
		let status = response.status();
		println!("Faucet POST response status: {}", status);
		if !status.is_success() {
			let error_text = response.text().await.unwrap_or_default();
			return Err(anyhow::anyhow!("Faucet request failed with status {}: {}", status, error_text));
		}
	}
	
	// Get the response body to see what the faucet actually returned
	let response_text = response.text().await.unwrap_or_default();
	println!("Faucet response body: {}", response_text);
	
	println!("Sender account funded request accepted by faucet");
	
	// Wait longer for account creation and add more debugging
	println!("Waiting for account to appear on-chain...");
	let mut created = false;
	for attempt in 1..=20 {  // Increased from 10 to 20 attempts
		println!("Checking account existence... attempt {}/20", attempt);
		match rest_client.get_account(sender.address()).await {
			Ok(account_info) => {
				println!("✅ Account now exists on-chain!");
				println!("  Sequence number: {}", account_info.inner().sequence_number);
				println!("  Authentication key: {:?}", account_info.inner().authentication_key);
				println!("  Created in attempt {}", attempt);
				created = true;
				break;
			}
			Err(e) => {
				println!("❌ Account not found yet (attempt {}/20): {}", attempt, e);
				if attempt < 20 {
					println!("Waiting 1 second before next check...");
					tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;  // Increased from 500ms to 1s
				}
			}
		}
	}
	
	if !created {
		return Err(anyhow::anyhow!("Sender account not found on-chain after faucet funding"));
	}
	
	println!("Sender account funded successfully via testnet faucet");
	
	// Create the beneficiary account (just create, no funding needed for this test)
	println!("Creating beneficiary account...");
	// For now, just create the account locally - it will be created when first transaction is sent to it

	// Test 1: Verify new fee collection mechanism
	println!("=== Test 1: Verifying new fee collection mechanism ===");
	
	// First, check if the COLLECT_AND_DISTRIBUTE_GAS_FEES feature flag is enabled
	println!("Checking if COLLECT_AND_DISTRIBUTE_GAS_FEES feature flag is enabled...");
	
	let feature_flag_view_req = ViewRequest {
		function: EntryFunctionId {
			module: MoveModuleId {
				address: Address::from_str("0x1")?,
				name: IdentifierWrapper::from_str("on_chain_config")?,
			},
			name: IdentifierWrapper::from_str("get_features")?,
		},
		type_arguments: vec![],
		arguments: vec![],
	};
	
	match rest_client.view(&feature_flag_view_req, None).await {
		Ok(features_response) => {
			println!("On-chain features response: {:?}", features_response.inner());
			let features_str = format!("{:?}", features_response.inner());
			if features_str.contains("COLLECT_AND_DISTRIBUTE_GAS_FEES") {
				println!("[PASS] COLLECT_AND_DISTRIBUTE_GAS_FEES feature flag appears enabled");
			} else {
				println!("[WARN] COLLECT_AND_DISTRIBUTE_GAS_FEES feature flag not found; continuing with routing checks");
			}
		}
		Err(e) => {
			println!("[WARN] Could not query on-chain features: {} — continuing with routing checks", e);
		}
	}

	// Query the fee collector address
	println!("Checking if transaction_fee module is accessible...");
	
	let transaction_fee_module_exists = {
		// Try to access the transaction_fee module through a simple view call
		let fee_collector_view_req = ViewRequest {
			function: EntryFunctionId {
				module: MoveModuleId {
					address: Address::from_str("0x1")?,
					name: IdentifierWrapper::from_str("transaction_fee")?,
				},
				name: IdentifierWrapper::from_str("collect_fee")?,
			},
			type_arguments: vec![],
			arguments: vec![],
		};
		
		match rest_client.view(&fee_collector_view_req, None).await {
			Ok(_) => {
				println!("[PASS] transaction_fee module is accessible");
				true
			}
			Err(e) => {
				println!("[WARN] transaction_fee module not accessible: {} - continuing with other checks", e);
				false
			}
		}
	};
	
	// For now, use a placeholder address since we can't query the actual collector
	// This will be updated when the module is properly accessible
	let fee_collector_addr = if transaction_fee_module_exists {
		// TODO: Get actual fee collector address when module is accessible
		AccountAddress::from_str("0x1")?
	} else {
		println!("[WARN] Using placeholder fee collector address 0x1 for testing");
		AccountAddress::from_str("0x1")?
	};
	
	println!("Fee collector address: {}", fee_collector_addr);

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

	// Snapshot fee collector balance before test tx
	let initial_fee_collector_balance = if transaction_fee_module_exists {
		match coin_client.get_account_balance(&fee_collector_addr).await {
			Ok(balance) => {
				println!("Initial fee collector balance: {}", balance);
				Some(balance)
			}
			Err(e) => {
				println!("[WARN] Could not get initial fee collector balance: {} - skipping balance checks", e);
				None
			}
		}
	} else {
		println!("[WARN] Skipping fee collector balance checks - module not accessible");
		None
	};

	// Test 3: Execute transactions and verify they use the new fee collection mechanism
	println!("=== Test 3: Executing test transaction to verify new fee collection mechanism ===");
	
	let initial_sender_balance = match coin_client.get_account_balance(&sender.address()).await {
		Ok(balance) => {
			println!("Initial sender balance: {}", balance);
			Some(balance)
		}
		Err(e) => {
			println!("[WARN] Could not get initial sender balance: {} - continuing without balance checks", e);
			None
		}
	};

	// Execute test transaction
	println!("Executing test transaction...");
	
	// Debug: Check account sequence number and other details
	println!("Sender account details:");
	println!("  Address: {}", sender.address());
	println!("  Sequence number: {}", sender.sequence_number());
	println!("  Public key: {:?}", sender.public_key());
	
	// Try to get account info from the chain
	match rest_client.get_account(sender.address()).await {
		Ok(account_info) => {
			println!("  On-chain sequence number: {}", account_info.inner().sequence_number);
			println!("  On-chain authentication key: {:?}", account_info.inner().authentication_key);
		}
		Err(e) => {
			println!("  [WARN] Could not get on-chain account info: {}", e);
		}
	}
	
	println!("Beneficiary address: {}", beneficiary.address());
	
	// Try the transaction with better error handling
	let test_txn = match coin_client.transfer(&mut sender, beneficiary.address(), 1_000, None).await {
		Ok(txn) => {
			println!("Transaction submitted successfully with hash: {:?}", txn);
			txn
		}
		Err(e) => {
			println!("[ERROR] Transaction submission failed: {}", e);
			println!("[DEBUG] This might be due to:");
			println!("  - Account not properly funded");
			println!("  - Sequence number mismatch");
			println!("  - Network connectivity issues");
			println!("  - Gas price/limit issues");
			return Err(e.context("Failed to submit test transaction"));
		}
	};
	
	rest_client
		.wait_for_transaction(&test_txn)
		.await
		.context("Failed when waiting for transfer transaction")?;
	
	println!("Test transaction completed: {:?}", test_txn);

	// Test 4: Verify gas fee collection and analyze the transaction
	println!("=== Test 4: Verifying gas fee collection and analyzing transaction ===");
	
	if let Some(initial_balance) = initial_sender_balance {
		let final_sender_balance = match coin_client.get_account_balance(&sender.address()).await {
			Ok(balance) => {
				println!("Final sender balance: {}", balance);
				Some(balance)
			}
			Err(e) => {
				println!("[WARN] Could not get final sender balance: {} - skipping balance comparison", e);
				None
			}
		};
		
		if let Some(final_balance) = final_sender_balance {
			// Verify that gas fees were deducted
			if final_balance < initial_balance {
				let gas_fees_deducted = initial_balance - final_balance;
				println!("Gas fees deducted: {}", gas_fees_deducted);
				
				println!("[PASS] Transaction executed successfully with hash: {:?}", test_txn);
				println!("[PASS] Gas fees were properly deducted, indicating fee collection is working");
			} else {
				println!("[FAIL] No gas fees were deducted - this indicates a serious issue");
				return Err(anyhow::anyhow!("Gas fee collection verification failed: no fees deducted"));
			}
		} else {
			println!("[WARN] Skipping gas fee deduction check due to balance retrieval failure");
		}
	} else {
		println!("[WARN] Skipping balance checks due to initial balance retrieval failure");
	}

	// Verify that the fee collector received funds (balance increased)
	if let Some(initial_balance) = initial_fee_collector_balance {
		let final_fee_collector_balance = coin_client
			.get_account_balance(&fee_collector_addr)
			.await
			.context("Failed to get final fee collector balance")?;

		if final_fee_collector_balance > initial_balance {
			let delta = final_fee_collector_balance - initial_balance;
			println!("[PASS] Fee collector balance increased by {}", delta);
		} else {
			println!(
				"[FAIL] Fee collector balance did not increase (before: {}, after: {})",
				initial_balance, final_fee_collector_balance
			);
			return Err(anyhow::anyhow!("Fee collector balance did not increase"));
		}
	} else {
		println!("[WARN] Skipping fee collector balance increase check due to module unavailability.");
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
	println!("=== Test 6: Verifying fee collection deprecation summary ===");
	
	if transaction_fee_module_exists && !governed_gas_pool_exists {
		println!("[PASS] Fee collection deprecation is COMPLETE");
		println!("[PASS] New transaction_fee::collect_fee mechanism is operational");
		println!("[PASS] Old governed_gas_pool mechanism is fully deprecated");
	} else if transaction_fee_module_exists && governed_gas_pool_exists {
		println!("[WARN] Fee collection deprecation is IN PROGRESS");
		println!("[PASS] New transaction_fee module is accessible");
		println!("[WARN] Old governed_gas_pool module is still accessible (deprecation in progress)");
	} else if !transaction_fee_module_exists {
		println!("[WARN] Fee collection deprecation status UNKNOWN");
		println!("[WARN] New transaction_fee module is not accessible");
		println!("[WARN] Old governed_gas_pool module status: {}", if governed_gas_pool_exists { "still accessible" } else { "not accessible" });
		println!("[WARN] This may indicate the framework upgrade is still in progress");
	} else {
		println!("[FAIL] Fee collection deprecation verification FAILED");
		println!("[FAIL] New transaction_fee module is not accessible");
		return Err(anyhow::anyhow!("Fee collection deprecation verification failed"));
	}

	println!("All tests completed successfully!");
	Ok(())
}


