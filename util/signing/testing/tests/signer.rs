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
		assert!(Ed25519.verify(message, &signature, &public_key)?);

		Ok(())
	}
}
