use crate::clap::eth_to_movement::EthSharedArgs;
use ethereum_bridge::client::Config;

impl From<EthSharedArgs> for Config {
	fn from(args: EthSharedArgs) -> Self {
		Self {
			rpc_url: args.eth_rpc_url,
			ws_url: args.eth_ws_url,
			signer_private_key: args.eth_private_key,
			initiator_contract: Some(args.eth_initiator_contract.0),
			counterparty_contract: Some(args.eth_counterparty_contract.0),
			weth_contract: args.eth_weth_contract.map(|x| x.0),
			gas_limit: args.eth_gas_limit,
		}
	}
}

impl From<&EthSharedArgs> for Config {
	fn from(args: &EthSharedArgs) -> Self {
		From::from(args.clone())
	}
}
