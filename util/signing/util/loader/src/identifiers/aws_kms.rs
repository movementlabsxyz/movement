use movement_signer::key::Key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsKms {
	pub key: Key,
}
