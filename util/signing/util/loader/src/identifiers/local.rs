use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Local {
	pub private_key_hex_bytes: String,
}
