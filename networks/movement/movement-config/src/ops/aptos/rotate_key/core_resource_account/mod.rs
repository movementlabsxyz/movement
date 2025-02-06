/// A helper struct to rotate the core resource account key.
pub struct RotateCoreResourceAccountKey;

impl RotateCoreResourceAccountKey {
	/// Creates a new instance of `RotateCoreResourceAccountKey`.
	pub fn new() -> Self {
		Self
	}

	pub async fn rotate_core_resource_account_key(
		&self,
		client: &aptos_sdk::rest_client::Client,
		old_signer: &impl ReleaseSigner,
		new_signer: &impl ReleaseSigner,
	) -> Result<Config, RotateCoreResourceAccountError> {
		let new_key = new_signer.public_key().to_bytes();
		let new_key = hex::encode(new_key);

		let mut config = Config::load()?;
		config.core_resource_account_key = new_key;
		config.save()?;

		Ok(config)
	}
}

/// Errors thrown by RotateCoreResourceAccount migrations.
#[derive(Debug, thiserror::Error)]
pub enum RotateCoreResourceAccountError {
	#[error("key rotation failed: {0}")]
	KeyRotationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait RotateCoreResourceAccountKeyOperations {
	/// Handles all side effects of rotating the core resource account key including writing to file and outputs a copy of the updated config.
	fn rotate_core_resource_account_key(
		&self,
		new_signer: &impl ReleaseSigner,
	) -> impl Future<Output = Result<Config, RotateCoreResourceAccountError>>;
}
