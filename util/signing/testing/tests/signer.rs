mod ed25519 {
	use movement_signer::cryptography::ed25519::{Ed25519, PublicKey};
	use movement_signer::{Signing, Verify};
	use movement_signer_test::ed25519::TestSigner;

	use ed25519_dalek::SigningKey;
	use rand::rngs::OsRng;

	#[tokio::test]
	async fn basic_signing() -> anyhow::Result<()> {
		let message = b"Hello, world!";
		let mut rng = OsRng;
		let signing_key = SigningKey::generate(&mut rng);
		let verifying_key = signing_key.verifying_key();
		let public_key = PublicKey::try_from(verifying_key.as_bytes() as &[u8])?;
		let signer = TestSigner::new(signing_key);

		let signature = signer.sign(message).await?;
		assert!(Ed25519::verify(message, &signature, &public_key)?);

		Ok(())
	}
}

mod secp256k1 {
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
	use movement_signer_aws_kms::hsm::AwsKms;
	use movement_signer_local::signer::LocalSigner;
	use movement_signing_eth::HsmSigner;
	use sha3::{Digest, Keccak256};
	use std::env;

	#[tokio::test]
	async fn local_signing_verify() -> Result<(), anyhow::Error> {
		let message = b"Hello, world!";
		let digest: [u8; 32] = Keccak256::new_with_prefix(&message).finalize().into();

		let signer = LocalSigner::<Secp256k1>::random();

		let public_key = signer.public_key().await?;
		let signature = signer.sign(&digest).await?;

		assert!(movement_signer::cryptography::secp256k1::Secp256k1::verify(
			&digest,
			&signature,
			&public_key
		)
		.unwrap());
		Ok(())
	}

	#[tokio::test]
	async fn aws_signing_verify() -> Result<(), anyhow::Error> {
		let message = b"Hello, world!";
		let digest: [u8; 32] = Keccak256::new_with_prefix(&message).finalize().into();
		///skip the test in Local mode
		if env::var("AWS_KMS_KEY_ID").is_ok() {
			let key_id = env::var("AWS_KMS_KEY_ID").expect("AWS_KMS_KEY_ID not set");
			let aws: AwsKms<Secp256k1> = AwsKms::try_from_env_with_key(key_id).await?;
			let public_key = aws.public_key().await?;
			let signature = aws.sign(&digest).await?;

			assert!(Secp256k1::verify(&digest, &signature, &public_key)?);
		}
		Ok(())
	}

	#[tokio::test]
	async fn send_tx() -> Result<(), anyhow::Error> {
		// Start Anvil
		let anvil = Anvil::new().port(8545u16).spawn();
		let rpc_url = anvil.endpoint_url();
		let chain_id = anvil.chain_id();

		// Detect if we execute with AWS env var or not.
		let (key_provider, address) = match env::var("AWS_KMS_KEY_ID") {
			Ok(key_id) => {
				// Use AWS KMS
				let _access_key = env::var("AWS_ACCESS_KEY").expect("AWS_ACCESS_KEY not set");
				let _secret_key = env::var("AWS_SECRET_KEY").expect("AWS_SECRET_KEY not set");
				let aws: AwsKms<Secp256k1> = AwsKms::try_from_env_with_key(key_id).await?;
				let signer = HsmSigner::try_new(aws, Some(chain_id)).await?;
				let address = signer.address();
				let key_provider = ProviderBuilder::new()
					.with_recommended_fillers()
					.wallet(EthereumWallet::new(signer))
					.on_builtin(&rpc_url.to_string())
					.await?;
				(key_provider, address)
			}
			Err(_) => {
				//Use Local KMS
				let local = LocalSigner::<Secp256k1>::random();
				let signer = HsmSigner::try_new(local, Some(chain_id)).await?;
				let address = signer.address();
				let key_provider = ProviderBuilder::new()
					.with_recommended_fillers()
					.wallet(EthereumWallet::new(signer))
					.on_builtin(&rpc_url.to_string())
					.await?;
				(key_provider, address)
			}
		};

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
			.with_value(U256::from(10_000_000_000_000_000u64));
		admin_provider.send_transaction(tx).await?.get_receipt().await?;

		let balance = key_provider.get_balance(address).await?;

		//transfer back some eth.
		let tx = TransactionRequest::default()
			.with_from(address)
			.with_to(admin_address)
			.with_value(U256::from(500))
			.with_chain_id(chain_id)
			.gas_limit(3_000_000);

		key_provider.send_transaction(tx).await?.get_receipt().await?;

		let new_balance = key_provider.get_balance(address).await?;
		assert!(
			balance != new_balance,
			"AWS account didn't change. Last transfer doesn't execute correctly."
		);

		Ok(())
	}
}
