use alloy_network::Ethereum;

use alloy_primitives::Address;
use alloy_primitives::Bytes;
use alloy_primitives::U256;
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::sol;
use alloy_transport::Transport;
use dot_movement::DotMovement;
use mcr_settlement_config::Config;
use std::future::Future;

mod local;

pub use local::Local;

/// Abstraction trait for MCR settlement setup strategies.
pub trait Setup {
	/// Sets up the MCR settlement client configuration.
	/// If required configuration values are unset, fills them with
	/// values decided by this setup strategy.
	fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send;
}

// Utility functions used by genesis ceremony setup.
// Load MRC smart contract ABI.
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MCR,
	"../client/abis/MCRLegacy.json"
);

pub async fn stake_genesis<P: Provider<T, Ethereum>, T: Transport + Clone>(
	provider: &P,
	contract: &MCR::MCRInstance<T, &P, Ethereum>,
	contract_address: Address,
	signer: Address,
	amount: u128,
) -> Result<(), anyhow::Error> {
	let stake_genesis_call = contract.stakeGenesis();
	let calldata = stake_genesis_call.calldata().to_owned();
	send_tx(provider, calldata, contract_address, signer, amount).await
}

pub async fn send_tx<P: Provider<T, Ethereum>, T: Transport + Clone>(
	provider: &P,
	call_data: Bytes,
	contract_address: Address,
	signer: Address,
	amount: u128,
) -> Result<(), anyhow::Error> {
	let eip1559_fees = provider.estimate_eip1559_fees(None).await?;
	let tx = TransactionRequest::default()
		.from(signer)
		.to(contract_address)
		.value(U256::from(amount))
		.input(call_data.into())
		.max_fee_per_gas(eip1559_fees.max_fee_per_gas)
		.max_priority_fee_per_gas(eip1559_fees.max_priority_fee_per_gas);

	provider.send_transaction(tx).await?.get_receipt().await?;
	Ok(())
}
