use movement_signer::key::{Key, TryFromCanonicalString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AwsKms {
	pub create: bool,
	pub key: Key,
}

impl TryFromCanonicalString for AwsKms {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		// split on the "::"
		let parts: Vec<&str> = s.split("::").collect();

		// if there are two parts, part 1 is whether or not the key should be created
		let create = if parts.len() == 2 {
			match parts[0] {
				"create" => true,
				_ => false,
			}
		} else {
			false
		};

		// if there are two parts, part 2 is the key
		let key = if parts.len() == 2 {
			Key::try_from_canonical_string(parts[1])?
		} else {
			Key::try_from_canonical_string(s)?
		};

		Ok(AwsKms { create, key })
	}
}
