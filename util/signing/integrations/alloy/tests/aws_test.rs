use alloy::node_bindings::Anvil;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_network::TransactionBuilder;
use alloy_network::TxSigner;
use alloy_primitives::U256;
use anyhow::Context;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::MessageType;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, SigningAlgorithmSpec};
use aws_sdk_kms::Client;
use movement_signing_alloy::HsmSigner;
use signer::{
	cryptography::secp256k1::Secp256k1, Bytes, PublicKey, Signature, SignerError, SignerOperations,
};
use std::env;

/// A AWS KMS HSM.
#[derive(Debug, Clone)]
pub struct AwsKms {
	pub client: Client,
	key_id: String,
}

#[async_trait::async_trait]
impl SignerOperations<Secp256k1> for AwsKms {
	/// Signs some bytes.
	async fn sign(&self, message: Bytes) -> Result<Signature, SignerError> {
		//println!("sign message {message:?}",);

		let res = self
			.client
			.sign()
			.key_id(&self.key_id)
			.message(Blob::new(message.0))
			.message_type(MessageType::Digest)
			.signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
			.send()
			.await
			.unwrap();

		//println!("sign res: {:?}", res);
		let signature = Signature(Bytes(
			res.signature().context("No signature available").unwrap().as_ref().to_vec(),
		));
		Ok(signature)
	}

	/// Gets the public key.
	async fn public_key(&self) -> Result<PublicKey, SignerError> {
		let res = self.client.get_public_key().key_id(&self.key_id).send().await.unwrap();
		//println!("public_key AWS KMS Response: {:?}", res);
		let public_key = PublicKey(Bytes(
			res.public_key().context("No public key available").unwrap().as_ref().to_vec(),
		));
		Ok(public_key)
	}
}

impl AwsKms {
	pub async fn new(key_id: String) -> Self {
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);
		AwsKms { client, key_id }
	}

	/// Creates in AWS KMS matching the provided key id.
	pub async fn create_key(&self) -> Result<String, anyhow::Error> {
		let res = self
			.client
			.create_key()
			.key_spec(KeySpec::EccSecgP256K1)
			.key_usage(KeyUsageType::SignVerify)
			.send()
			.await?;

		let key_id = res.key_metadata().context("No key metadata available")?.key_id().to_string();

		Ok(key_id)
	}

	pub fn set_key_id(&mut self, key_id: String) {
		self.key_id = key_id;
	}
}

#[tokio::test]
async fn test_aws_kms_send_tx() -> Result<(), anyhow::Error> {
	// Start Anvil
	let mut anvil = Anvil::new().port(8545u16).arg("-vvvvv").spawn();
	let rpc_url = anvil.endpoint_url();
	let chain_id = anvil.chain_id();

	// Use AWS KMS
	let _access_key = env::var("AWS_ACCESS_KEY").expect("AWS_ACCESS_KEY not set");
	let _secret_key = env::var("AWS_SECRET_KEY").expect("AWS_SECRET_KEY not set");
	let key_id = env::var("AWS_KEY_ID").expect("AWS_KEY_ID not set");

	println!("key_id:{key_id}");

	let aws = AwsKms::new(key_id).await;
	let signer = HsmSigner::new(Box::new(aws), Some(chain_id)).await?;
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
