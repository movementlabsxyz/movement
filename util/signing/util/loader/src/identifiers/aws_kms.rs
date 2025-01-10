use movement_signer::key::Key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AwsKms {
	pub key: Key,
}
