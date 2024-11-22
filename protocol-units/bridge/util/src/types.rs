use alloy::primitives::Uint;
use alloy::serde::quantity::vec;
use derive_more::{Deref, DerefMut};
use hex::{self, FromHexError};
use rand::Rng;
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt;
use std::{fmt::Debug, hash::Hash};
use thiserror::Error;

pub type BridgeHash = [u8; 32];

#[derive(Debug, Error)]
pub enum AddressError {
	#[error("Invalid hex string")]
	InvalidHexString,
	#[error("Invalid byte length for AccountAddress: {0}")]
	InvalidByteLength(usize),
	#[error("Invalid AccountAddress: {0}")]
	AccountParseError(String),
	#[error("Error during address conversion: {0}")]
	AddressConvertionlError(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ChainId {
	ONE,
	TWO,
}

impl ChainId {
	pub fn other(&self) -> ChainId {
		match self {
			ChainId::ONE => ChainId::TWO,
			ChainId::TWO => ChainId::ONE,
		}
	}
}

impl fmt::Display for ChainId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let s = match self {
			ChainId::ONE => "ONE",
			ChainId::TWO => "TWO",
		};
		write!(f, "{}", s)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct BridgeTransferId(pub BridgeHash);

impl BridgeTransferId {
	pub fn parse(s: &str) -> Result<Self, FromHexError> {
		let bytes = hex::decode(s)?;
		let array: [u8; 32] =
			bytes.as_slice().try_into().map_err(|_| FromHexError::InvalidStringLength)?;
		Ok(BridgeTransferId(array))
	}
	pub fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self {
		let mut random_bytes = [0u8; 32];
		rng.fill(&mut random_bytes);
		BridgeTransferId(random_bytes)
	}

	pub fn test() -> Self {
		let array = [0u8; 32];
		BridgeTransferId(array)
	}
}

impl TryFrom<Vec<u8>> for BridgeTransferId {
	type Error = Vec<u8>;

	fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(BridgeTransferId(data.try_into()?))
	}
}

impl fmt::Display for BridgeTransferId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Bid: {}", hex::encode(self.0))
	}
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct BridgeAddress<A>(pub A);

impl BridgeAddress<Vec<u8>> {
	pub fn test() -> Self {
		let array = [0u8; 32];
		BridgeAddress(array.to_vec())
	}
}

impl From<&str> for BridgeAddress<Vec<u8>> {
	fn from(value: &str) -> Self {
		Self(value.as_bytes().to_vec())
	}
}

impl From<String> for BridgeAddress<Vec<u8>> {
	fn from(value: String) -> Self {
		Self(value.as_bytes().to_vec())
	}
}

#[derive(Error, Debug)]
pub enum BridgeAddressError {
	#[error("Invalid conversion from BridgeAddress to Vec<u8>")]
	InvalidConversion,
}

pub trait ToCommonAddress: Sized {
	fn to_common_address(self) -> Result<BridgeAddress<Vec<u8>>, BridgeAddressError> {
		Err(BridgeAddressError::InvalidConversion)
	}
}

impl<A> ToCommonAddress for BridgeAddress<A>
where
	A: Into<Vec<u8>>,
{
	fn to_common_address(self) -> Result<BridgeAddress<Vec<u8>>, BridgeAddressError> {
		Ok(BridgeAddress(self.0.into()))
	}
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct HashLock(pub [u8; 32]);

impl HashLock {
	pub fn parse(s: &str) -> Result<Self, FromHexError> {
		let bytes = hex::decode(s)?;
		let array: [u8; 32] =
			bytes.as_slice().try_into().map_err(|_| FromHexError::InvalidStringLength)?;
		Ok(HashLock(array))
	}
	/// Generate a cryptographically secure random secret
	pub fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut secret = [0u8; 32];
		rng.fill(&mut secret);
		HashLock(secret)
	}

	pub fn test() -> Self {
		let array = [0u8; 32];
		HashLock(array)
	}
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct HashLockPreImage(pub [u8; 32]);

impl AsRef<[u8]> for HashLockPreImage {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl HashLockPreImage {
	/// Generate a cryptographically secure random secret
	pub fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut secret = [0u8; 32];
		rng.fill(&mut secret);
		HashLockPreImage(secret)
	}
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct TimeLock(pub u64);

impl From<Uint<256, 4>> for TimeLock {
	fn from(value: Uint<256, 4>) -> Self {
		// Extract the lower 64 bits.
		let lower_64_bits = value.as_limbs()[0];
		TimeLock(lower_64_bits)
	}
}

#[derive(Deref, DerefMut, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Amount(pub u64);

impl From<Uint<256, 4>> for Amount {
	fn from(value: Uint<256, 4>) -> Self {
		// Extract the lower 64 bits.
		let lower_64_bits = value.as_limbs()[0];
		Amount(lower_64_bits)
	}
}

#[derive(Error, Debug)]
pub enum ConversionError {
	#[error("Invalid conversion from AssetType to Uint")]
	InvalidConversion,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct BridgeTransferDetails<A> {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator_address: BridgeAddress<A>,
	pub recipient_address: BridgeAddress<Vec<u8>>,
	pub hash_lock: HashLock,
	pub time_lock: TimeLock,
	pub amount: Amount,
	pub state: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct BridgeTransferDetailsCounterparty<A> {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator_address: BridgeAddress<Vec<u8>>,
	pub recipient_address: BridgeAddress<A>,
	pub hash_lock: HashLock,
	pub time_lock: TimeLock,
	pub amount: Amount,
	pub state: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct LockDetails<A> {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator: BridgeAddress<Vec<u8>>,
	pub recipient: BridgeAddress<A>,
	pub hash_lock: HashLock,
	pub time_lock: TimeLock,
	pub amount: Amount,
}
