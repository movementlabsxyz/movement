use crate::types::AddressError;

pub mod bridge_contracts;

pub trait AddressVecCodec: TryFrom<Vec<u8>, Error = AddressError> + Into<Vec<u8>> {
	fn try_decode(value: Vec<u8>) -> Result<Self, AddressError>;
	fn encode(self) -> Vec<u8>;
}
