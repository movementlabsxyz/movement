use bridge_config::Config;
use std::process::{Command, Stdio};
use anyhow::{Result, Context};
use serde_json::Value;

#[tokio::test]
async fn test_lock_mint() -> Result<()> {
    // Define bridge config path
    let mock_config = Config::default();

    // tracing::info!("Before compile move modules");
	// let compile_output = Command::new("movement")
	// 	.args(&["move", "compile", "--package-dir", "protocol-units/bridge/move-modules/"])
	// 	.stdout(Stdio::piped())
	// 	.stderr(Stdio::piped())
	// 	.output()?;

	// if !compile_output.stdout.is_empty() {
	// 	tracing::info!("move compile stdout: {}", String::from_utf8_lossy(&compile_output.stdout));
	// }
	// if !compile_output.stderr.is_empty() {
	// 	tracing::info!("move compile stderr: {}", String::from_utf8_lossy(&compile_output.stderr));
	// }
	// let enable_bridge_feature_output = Command::new("movement")
	// 		.args(&[
	// 			"move",
	// 			"run-script",
	// 			"--compiled-script-path",
	// 			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv",
	// 			"--profile",
	// 			"default",
	// 			"--assume-yes",
	// 		])
	// 		.stdout(Stdio::piped())
	// 		.stderr(Stdio::piped())
	// 		.output()?;

	// if !enable_bridge_feature_output.stdout.is_empty() {
	// 	println!(
	// 		"run-script enable_bridge_feature stdout: {}",
	// 		String::from_utf8_lossy(&enable_bridge_feature_output.stdout)
	// 	);
	// }
	// if !enable_bridge_feature_output.stderr.is_empty() {
	// 	eprintln!(
	// 		"run-script enable_bridge_feature stderr: {}",
	// 		String::from_utf8_lossy(&enable_bridge_feature_output.stderr)
	// 	);
	// }

	// let store_mint_burn_caps_output = Command::new("movement")
	// 		.args(&[
	// 			"move",
	// 			"run-script",
	// 			"--compiled-script-path",
	// 			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/store_mint_burn_caps.mv",
	// 			"--profile",
	// 			"default",
	// 			"--assume-yes",
	// 		])
	// 		.stdout(Stdio::piped())
	// 		.stderr(Stdio::piped())
	// 		.output()?;

	// if !store_mint_burn_caps_output.stdout.is_empty() {
	// 	println!(
	// 		"run-script store_mint_burn_caps stdout: {}",
	// 		String::from_utf8_lossy(&store_mint_burn_caps_output.stdout)
	// 	);
	// }
	// if !store_mint_burn_caps_output.stderr.is_empty() {
	// 	eprintln!(
	// 		"run-script store_mint_burn_caps stderr: {}",
	// 		String::from_utf8_lossy(&store_mint_burn_caps_output.stderr)
	// 	);
	// }

	// let update_bridge_relayer_output = Command::new("movement")
	// 		.args(&[
	// 			"move",
	// 			"run-script",
	// 			"--compiled-script-path",
	// 			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/update_bridge_relayer.mv",
	// 			"--args",
	// 			"address:0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16",
	// 			"--profile",
	// 			"default",
	// 			"--assume-yes",
	// 		])
	// 		.stdout(Stdio::piped())
	// 		.stderr(Stdio::piped())
	// 		.output()?;

	// if !update_bridge_relayer_output.stdout.is_empty() {
	// 	println!(
	// 		"run-script update_bridge_relayer stdout: {}",
	// 		String::from_utf8_lossy(&update_bridge_relayer_output.stdout)
	// 	);
	// }
	// if !update_bridge_relayer_output.stderr.is_empty() {
	// 	eprintln!(
	// 		"run-script update_bridge_relayer update_bridge_relayer stderr: {}",
	// 		String::from_utf8_lossy(&update_bridge_relayer_output.stderr)
	// 	);
	// }

    tracing::info!("sending 1 coin to dead");

    // Transfer 1 coin to 0x...dead
    Command::new("movement")
        .args(&[
            "move",
            "run",
            "--function-id",
            "0x1::aptos_account::transfer",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
            "u64:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to mint 0x000000000000000000000000000000000000000000000000000000000000dead");

    tracing::info!("get dead balance");

    // Get 0x...dead balance
    let dead_balance_output = Command::new("movement")
        .args(&[
            "move",
            "view",
            "--function-id",
            "0x1::coin::balance",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to get balance")?;

    let dead_balance_str = extract_result_value(&dead_balance_output.stdout)?;
    let dead_balance: u64 = dead_balance_str.parse().context("Failed to parse dead balance")?;

    tracing::info!("burn dead balance");

    // Burn 0x...dead balance
    Command::new("movement")
        .args(&[
            "move",
            "run",
            "--function-id",
            "0x1::coin::burn_from",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
            &format!("u64:{}", dead_balance),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to burn 0x...dead balance");

    tracing::info!("get bridge relayer");

    // Get the bridge relayer address
    // let bridge_relayer_output = Command::new("movement")
    //     .args(&[
    //         "move",
    //         "view",
    //         "--function-id",
    //         "0x1::native_bridge::get_bridge_relayer",
    //     ])
    //     .stdin(Stdio::piped())
    //     .stdout(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .output()
    //     .context("Failed to view bridge relayer")?;

    // let bridge_relayer = extract_result_value(&bridge_relayer_output.stdout)?;

    // tracing::info!("get bridge relayer balance");

    // let balance_output = Command::new("movement")
    //     .args(&[
    //         "move",
    //         "view",
    //         "--function-id",
    //         "0x1::coin::balance",
    //         "--type-args",
    //         "0x1::aptos_coin::AptosCoin",
    //         "--args",
    //         &format!("address:{}", bridge_relayer),
    //     ])
    //     .stdin(Stdio::piped())
    //     .stdout(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .output()
    //     .context("Failed to get balance")?;

    // let balance_str = extract_result_value(&balance_output.stdout)?;
    // let balance: u64 = balance_str.parse().context("Failed to parse bridge relayer balance")?;
    // let desired_balance = 1_000_000_000_000_000;
    // let burn_balance = balance.saturating_sub(desired_balance);

    // tracing::info!("burn excess");
    // println!("burn excess");
    // if burn_balance > 0 {
    //     Command::new("movement")
    //         .args(&[
    //             "move",
    //             "run",
    //             "--function-id",
    //             "0x1::coin::burn_from",
    //             "--type-args",
    //             "0x1::aptos_coin::AptosCoin",
    //             "--args",
    //             &format!("address:{}", bridge_relayer),
    //             &format!("u64:{}", burn_balance),
    //         ])
    //         .stdin(Stdio::piped())
    //         .stdout(Stdio::piped())
    //         .stderr(Stdio::piped())
    //         .spawn()
    //         .expect("Failed to burn excess");
    // }

    Ok(())
}

// Helper function to extract `Result` value from JSON output
fn extract_result_value(output: &[u8]) -> Result<String> {
    // Parse the JSON output
    let json: Value = serde_json::from_slice(output).context("Failed to parse JSON")?;
    println!("Parsed JSON: {:#?}", json); // Debugging parsed JSON

    // Extract the `Result` field
    json.get("Result")
        .and_then(|res| {
            if res.is_array() {
                res.as_array()
                    .and_then(|arr| arr.get(0)) // Get the first element
                    .and_then(|val| val.as_str()) // Convert to a string
            } else {
                res.as_str() // If it's not an array, try to read it as a string directly
            }
        })
        .map(|s| s.to_string())
        .context("Result field not found or invalid format")
}