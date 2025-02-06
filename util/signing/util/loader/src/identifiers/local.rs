use movement_signer::key::TryFromCanonicalString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Local {
	pub private_key_hex_bytes: String,
}

impl TryFromCanonicalString for Local {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		Ok(Local { private_key_hex_bytes: s.to_string() })
	}
}
