use crate::types::AddressError;

pub mod bridge_contracts;

// Address are converted from one chain to another.
// Source chain Initiator address are converted in a Vec<u8> for target Chain.
// Source chain Recipient address is a Vec<u8> that is converted to target address.
pub trait AddressVecCodec: TryFrom<Vec<u8>, Error = AddressError> + Into<Vec<u8>> {
	fn try_decode_recipient(value: Vec<u8>) -> Result<Self, AddressError>;
	fn encode_initiator(self) -> Vec<u8>;
}
