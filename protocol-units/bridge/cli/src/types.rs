use crate::clap::eth_to_movement::EthSharedArgs;
use ethereum_bridge::Config;

impl From<EthSharedArgs> for Config {
	fn from(args: EthSharedArgs) -> Self {
		Self {
			rpc_url: args.eth_rpc_url,
			ws_url: args.eth_ws_url,
			signer_private_key: args.eth_signer_private_key,
			initiator_contract: args.eth_initiator_contract,
			counterparty_contract: args.eth_counterparty_contract,
			gas_limit: args.eth_gas_limit,
		}
	}
}

impl From<&EthSharedArgs> for Config {
	fn from(args: &EthSharedArgs) -> Self {
		From::from(args.clone())
	}
}
