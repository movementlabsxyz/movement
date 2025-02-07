pub mod aws_kms;
pub mod hashi_corp_vault;
pub mod local;

use anyhow::anyhow;
use movement_signer::{cryptography::Curve, key::TryFromCanonicalString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum SignerIdentifier {
	Local(local::Local),
	AwsKms(aws_kms::AwsKms),
	HashiCorpVault(hashi_corp_vault::HashiCorpVault),
}

impl SignerIdentifier {
	pub fn to_typed<C>(self) -> TypedSignerIdentifier<C>
	where
		C: Curve,
	{
		TypedSignerIdentifier::new(self)
	}

	/// Returns the bytes of the private key if it can be extracted from
	/// the identifier. This method is only used for testing.
	pub fn try_raw_private_key(&self) -> Result<Vec<u8>, anyhow::Error> {
		match self {
			SignerIdentifier::Local(local::Local { private_key_hex_bytes }) => {
				let key = hex::decode(private_key_hex_bytes)?;
				Ok(key)
			}
			_ => Err(anyhow!("")),
		}
	}
}

impl TryFromCanonicalString for SignerIdentifier {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		// split on the first "::"
		let parts: Vec<&str> = s.splitn(2, "::").collect();

		if parts.len() < 2 {
			return Err("invalid signer identifier".to_string());
		}

		match parts[0] {
			"local" => {
				Ok(SignerIdentifier::Local(local::Local::try_from_canonical_string(parts[1])?))
			}
			"aws_kms" => {
				Ok(SignerIdentifier::AwsKms(aws_kms::AwsKms::try_from_canonical_string(parts[1])?))
			}
			"hashi_corp_vault" => Ok(SignerIdentifier::HashiCorpVault(
				hashi_corp_vault::HashiCorpVault::try_from_canonical_string(parts[1])?,
			)),
			_ => Err("invalid signer identifier".to_string()),
		}
	}
}

pub struct TypedSignerIdentifier<C>
where
	C: Curve,
{
	pub signer_identifier: SignerIdentifier,
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> TypedSignerIdentifier<C>
where
	C: Curve,
{
	pub fn new(signer_identifier: SignerIdentifier) -> Self {
		Self { signer_identifier, __curve_marker: std::marker::PhantomData }
	}
}
