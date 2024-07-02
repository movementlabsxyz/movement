use anyhow::Result;
//use bridge_shared::types::BridgeTransferId;
//use std::process::Command;
use hex::encode;
use std::{env, process::Stdio};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
	let config_path = "path/to/config.json";
	// 1st Anvil test address
	let initiator_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
	let recipient_address = "0x123"; // dummy val, this should be a movement address

	let hash_lock = "forty-two".as_bytes();
	let hash_lock_bytes = keccak_hash::keccak(hash_lock);
	let hash_lock = encode(hash_lock_bytes);

	let time_lock: u64 = 3600; // Example value
	let amount: u64 = 1000; // Example value
	let bridge_transfer_id = "bridge_transfer_id";
	let pre_image = "pre_image";

	let current_dir = env::current_dir()?;
	println!("Current dir: {:?}", current_dir);

	let build_output = TokioCommand::new("forge")
		.args(&["build"])
		.current_dir("protocol-units/bridge/contracts") //navigate to contracts dir
		.output()
		.await?;
	if !build_output.status.success() {
		eprint!("Failed to build contracts: {}", String::from_utf8_lossy(&build_output.stderr));
		return Err(anyhow::anyhow!("Failed to build contracts"));
	} else {
		println!("forge build output:");
		println!("{}", String::from_utf8_lossy(&build_output.stdout));
	}

	let _ = TokioCommand::new("anvil").stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
	sleep(Duration::from_secs(5)).await;

	// let deploy_status = TokioCommand::new("forge")
	// 	.args(&["-c", "anvil", "-p", "anvil"])
	// 	.status()
	// 	.await?;
	// assert!(deploy_status.success(), "Deployment failed");
	//
	sleep(Duration::from_secs(5)).await;

	// Step 1: Deploy
	println!("Deploying contract...");
	let deploy_status = TokioCommand::new("cargo")
		.args(&[
			"run",
			"--package",
			"bridge-cli",
			"--",
			"eth",
			"deploy",
			"--config-path",
			config_path,
		])
		.status()
		.await?;
	assert!(deploy_status.success(), "Deployment failed");

	// Step 2: Initiate
	println!("Initiating transfer...");
	let initiate_status = TokioCommand::new("cargo")
		.args(&[
			"run",
			"--package",
			"bridge-cli",
			"--",
			"eth",
			"initiate",
			"--config-path",
			config_path,
			"--initiator-address",
			initiator_address,
			"--recipient-address",
			recipient_address,
			"--hash-lock",
			hash_lock.as_str(),
			"--time-lock",
			&time_lock.to_string(),
			"--amount",
			&amount.to_string(),
		])
		.status()
		.await?;
	assert!(initiate_status.success(), "Initiation failed");

	// Step 3: Complete
	println!("Completing transfer...");
	let complete_status = TokioCommand::new("cargo")
		.args(&[
			"run",
			"--package",
			"bridge-cli",
			"--",
			"eth",
			"complete",
			"--config-path",
			config_path,
			"--bridge-transfer-id",
			bridge_transfer_id,
			"--pre-image",
			pre_image,
		])
		.status()
		.await?;
	assert!(complete_status.success(), "Completion failed");

	println!("E2E flow completed successfully");
	Ok(())
}
