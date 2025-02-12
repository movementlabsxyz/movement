use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signing_aptos::key_rotation::signer::KeyRotationSigner;

/// [CoreResourceAccountKeyRotationSigner] has to be a [KeyRotationSigner] that can also return an identifier for the signer.
pub trait CoreResourceAccountKeyRotationSigner: KeyRotationSigner {
	/// Returns the identifier for the signer.
	fn signer_identifier(&self) -> SignerIdentifier;
}
