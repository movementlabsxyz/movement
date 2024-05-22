pub use thiserror::Error;
pub mod v1;
pub use m1_da_light_node_grpc::*;

pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Error, Debug)]
pub enum VerifierError {
	#[error("Failed to verify proof")]
	VerifyProofFailed,
	#[error("Missing root hash")]
	MissingRootHash,
	#[error("Other: $0")]
	Other(#[from] BoxedError),
}

pub fn to_verifier_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> VerifierError {
	VerifierError::Other(Box::new(e))
}

pub trait ToVerifierError<T> {
	fn map_verifier_err(self) -> Result<T, VerifierError>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ToVerifierError<T> for Result<T, E> {
	fn map_verifier_err(self) -> Result<T, VerifierError> {
		self.map_err(to_verifier_error)
	}
}

#[tonic::async_trait]
pub trait Verifier {
	async fn verify(
		&self,
		verification_mode: VerificationMode,
		blob: &[u8],
		height: u64,
	) -> Result<bool, VerifierError> {
		match verification_mode {
			VerificationMode::Cowboy => self.verify_cowboy(verification_mode, blob, height).await,
			VerificationMode::ValidatorIn => {
				self.verifiy_validator_in(verification_mode, blob, height).await
			},
			VerificationMode::MOfN => self.verify_m_of_n(verification_mode, blob, height).await,
		}
	}

	async fn verify_cowboy(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError> {
		Ok(true)
	}

	async fn verifiy_validator_in(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError>;

	async fn verify_m_of_n(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError>;
}

