use movement_signer::key::Key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HashiCorpVault {
	pub key: Key,
}
