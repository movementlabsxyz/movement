use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use clap::Parser;
use clap::Subcommand;
use k256::ecdsa::VerifyingKey;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer::key::TryFromCanonicalString;
use movement_signer::Signing;
use movement_signer::Verify;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signer_loader::{Load, LoadedSigner};

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands to test key name")]
pub enum TestKey {
	Ed25519(TestKeyParam),
	Secp256k1(TestKeyParam),
}

impl TestKey {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			TestKey::Ed25519(param) => param.execute_ed25519().await,
			TestKey::Secp256k1(param) => param.execute_secp256k1().await,
		}
	}
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Key to test.")]
pub struct TestKeyParam {
	#[clap(default_value = "{maptos,maptos-storage,movement-da-db}/**", value_name = "DB PATTERN")]
	pub name: String,
}

impl TestKeyParam {
	pub async fn execute_ed25519(&self) -> Result<(), anyhow::Error> {
		let signer_identifier = SignerIdentifier::try_from_canonical_string(&self.name)
			.map_err(|err| anyhow::anyhow!(err))?;
		let loader: LoadedSigner<Ed25519> = signer_identifier.load().await?;

		let public_key = Ed25519PublicKey::try_from(loader.public_key().await?.as_bytes())?;
		let account_address = AuthenticationKey::ed25519(&public_key).account_address();

		tracing::info!("Key loaded, account address:{account_address}");
		tracing::info!("Try to sign a message ...");

		let message = b"Hello, world!";
		let signature = loader.sign(message).await?;
		assert!(Ed25519::verify(message, &signature, &loader.public_key().await?)?);

		tracing::info!("Message sign verify pass");

		Ok(())
	}
	pub async fn execute_secp256k1(&self) -> Result<(), anyhow::Error> {
		let signer_identifier = SignerIdentifier::try_from_canonical_string(&self.name)
			.map_err(|err| anyhow::anyhow!(err))?;
		let loader: LoadedSigner<Secp256k1> = signer_identifier.load().await?;
		let pub_key = loader.public_key().await?;
		let verify_key = VerifyingKey::from_sec1_bytes(pub_key.as_bytes())?;

		let account_address = alloy_signer::utils::public_key_to_address(&verify_key);

		tracing::info!("Key loaded, account address:{account_address}");
		tracing::info!("Try to sign a message ...");

		let message = b"Hello, world!";
		let signature = loader.sign(message).await?;
		assert!(Secp256k1::verify(message, &signature, &loader.public_key().await?)?);

		tracing::info!("Message sign verify pass");

		Ok(())
	}
}
