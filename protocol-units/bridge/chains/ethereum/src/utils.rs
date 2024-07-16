//NB Only one funcition here now but in the event logging
//PR more is to be added.

use alloy::pubsub::PubSubFrontend;
use alloy_primitives::Address;
use alloy_provider::RootProvider;

use crate::AlloyProvider;

pub(crate) struct ProviderArgs {
	pub rpc_provider: AlloyProvider,
	pub ws_provider: RootProvider<PubSubFrontend>,
	pub initiator_address: Address,
	pub signer_address: Address,
	pub counterparty_address: Address,
	pub gas_limit: u64,
	pub num_tx_send_retries: u32,
	pub chain_id: String,
}

pub fn vec_to_array(vec: Vec<u8>) -> Result<[u8; 32], &'static str> {
	if vec.len() == 32 {
		// Try to convert the Vec<u8> to [u8; 32]
		match vec.try_into() {
			Ok(array) => Ok(array),
			Err(_) => Err("Failed to convert Vec<u8> to [u8; 32]"),
		}
	} else {
		Err("Vec<u8> does not have exactly 32 elements")
	}
}
