use alloy::node_bindings::Anvil;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_network::TransactionBuilder;
use alloy_network::TxSigner;
use alloy_primitives::U256;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer::Signing;
use movement_signer::Verify;
use movement_signer_aws_kms::hsm::AwsKmsSigner;
use movement_signing_eth::HsmSigner;
use sha3::{Digest, Keccak256};
use std::env;

#[tokio::test]
async fn basic_signing_verify() -> Result<(), anyhow::Error> {
	let message = b"Hello, world!";
	let digest: [u8; 32] = Keccak256::new_with_prefix(&message).finalize().into();
	let key_id = env::var("AWS_KEY_ID").expect("AWS_KEY_ID not set");
	let aws = AwsKmsSigner::new(key_id).await;
	let public_key = aws.public_key().await?;
	let signature = aws.sign(&digest).await?;

	assert!(Secp256k1.verify(&digest, &signature, &public_key)?);
	Ok(())
}

#[tokio::test]
async fn test_aws_kms_send_tx() -> Result<(), anyhow::Error> {
	// Start Anvil
	let anvil = Anvil::new().port(8545u16).arg("-vvvvv").spawn();
	let rpc_url = anvil.endpoint_url();
	let chain_id = anvil.chain_id();

	// Use AWS KMS
	let _access_key = env::var("AWS_ACCESS_KEY").expect("AWS_ACCESS_KEY not set");
	let _secret_key = env::var("AWS_SECRET_KEY").expect("AWS_SECRET_KEY not set");
	let key_id = env::var("AWS_KEY_ID").expect("AWS_KEY_ID not set");

	println!("key_id:{key_id}");

	let aws = AwsKmsSigner::new(key_id).await;
	let signer = HsmSigner::try_new(aws, Some(chain_id)).await?;
	let address = signer.address();
	println!("DEEEEB Key address:{}", address);

	let key_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::new(signer))
		.on_builtin(&rpc_url.to_string())
		.await?;

	let admin: PrivateKeySigner = anvil.keys()[1].clone().into();
	let admin_address = admin.address();
	let admin_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::new(admin))
		.on_builtin(&rpc_url.to_string())
		.await?;

	//transfer some eth to the key.
	let tx = TransactionRequest::default()
		.with_to(address)
		.with_value(U256::from(1000000000));
	let receipt = admin_provider.send_transaction(tx).await?.get_receipt().await?;
	println!("Admin -> Key receipt: {receipt:?}",);

	let account = key_provider.get_accounts().await;
	println!("Account: {:?}", account);
	let balance = key_provider.get_balance(address).await;
	println!("Balance: {:?}", balance);

	//transfer back some eth.
	let tx = TransactionRequest::default()
		.with_from(address)
		.with_to(admin_address)
		.with_value(U256::from(5))
		.gas_limit(3000000);
	println!("Tx from {:?}", tx.from);

	let receipt = key_provider.send_transaction(tx).await; //.get_receipt().await?;
	println!("Key -> Admin receipt: {receipt:?}",);

	// // Print ANvil output.
	// use std::io;
	// use std::io::BufRead;
	// use std::io::BufReader;
	// use std::io::Write;

	// let anvil_out = anvil.child_mut().stdout.take().unwrap();
	// let mut stdout_writer = io::stdout();
	// let mut reader = BufReader::new(anvil_out).lines();
	// while let Some(Ok(line)) = reader.next() {
	// 	stdout_writer.write_all(line.as_bytes())?;
	// 	stdout_writer.write_all(b"\n")?;
	// }

	Ok(())
}
