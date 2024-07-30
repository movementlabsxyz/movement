pub mod v1;

pub use m1_da_light_node_grpc::*;

#[tonic::async_trait]
pub trait Verifier {
	async fn verify(
		&self,
		verification_mode: VerificationMode,
		blob: &[u8],
		height: u64,
	) -> Result<bool, anyhow::Error> {
		match verification_mode {
			VerificationMode::Cowboy => self.verify_cowboy(verification_mode, blob, height).await,
			VerificationMode::ValidatorIn => {
				self.verify_validator_in(verification_mode, blob, height).await
			}
			VerificationMode::MOfN => self.verify_m_of_n(verification_mode, blob, height).await,
		}
	}

	async fn verify_cowboy(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, anyhow::Error> {
		Ok(true)
	}

	async fn verify_validator_in(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, anyhow::Error>;

	async fn verify_m_of_n(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, anyhow::Error>;
}
