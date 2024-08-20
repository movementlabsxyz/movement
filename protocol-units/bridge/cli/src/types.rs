use crate::clap::eth_to_movement::{EthSharedArgs, MoveSharedArgs};
use ethereum_bridge::client::Config;
use movement_bridge::Config as MovementConfig;

impl From<&EthSharedArgs> for Config {
	fn from(args: &EthSharedArgs) -> Self {
		From::from(args.clone())
	}
}

impl From<EthSharedArgs> for Config {
	fn from(args: EthSharedArgs) -> Self {
		Self {
			rpc_url: args.eth_rpc_url,
			ws_url: args.eth_ws_url,
			signer_private_key: args.eth_signer_private_key,
			initiator_contract: Some(args.eth_initiator_contract.0),
			counterparty_contract: Some(args.eth_counterparty_contract.0),
			gas_limit: args.eth_gas_limit,
		}
	}
}

impl From<&MoveSharedArgs> for MovementConfig {
	fn from(args: &MoveSharedArgs) -> Self {
		From::from(args.clone())
	}
}

impl From<MoveSharedArgs> for MovementConfig {
	fn from(args: MoveSharedArgs) -> Self {
		// turn a private key string into an Arc<RwLock<LocalAccount>> with ind
		Self {
			rpc_url: Some(args.move_rpc_url),
			ws_url: Some(args.move_ws_url),
			chain_id: args.move_chain_id,
			signer_private_key: args.move_signer_private_key,
			initiator_contract: Some(args.move_initiator_contract),
			counterparty_contract: Some(args.move_counterparty_contract),
			gas_limit: args.move_gas_limit,
		}
	}
}


