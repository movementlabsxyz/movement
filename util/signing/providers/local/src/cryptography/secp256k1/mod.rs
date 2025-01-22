use crate::cryptography::LocalCryptographySpec;
use movement_signer::cryptography::secp256k1::Secp256k1;

impl LocalCryptographySpec for Secp256k1 {
	type Curve = k256::Secp256k1;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::signer::LocalSigner;
	use movement_signer::{Signing, Verify};

	#[tokio::test]
	pub async fn test_signs_and_verifies() -> Result<(), anyhow::Error> {
		let signer = LocalSigner::<Secp256k1>::random();
		let message = b"hello world";
		let signature = signer.sign(message).await?;
		let public_key = signer.public_key().await?;

		assert!(Secp256k1::verify(message, &signature, &public_key)?);

		Ok(())
	}
}
