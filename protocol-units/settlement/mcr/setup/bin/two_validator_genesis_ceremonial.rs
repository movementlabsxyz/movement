use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use mcr_settlement_setup::stake_genesis;
use mcr_settlement_setup::MCR;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	//load local env.
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let suzuka_config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	let mcr_address: Address = suzuka_config.mcr.mcr_contract_address.parse()?;
	do_genesis_ceremonial_one_validator(
		mcr_address,
		&suzuka_config.mcr.test_local.as_ref().unwrap().anvil_keys,
		&suzuka_config.mcr.rpc_url.as_ref().unwrap(),
	)
	.await?;
	Ok(())
}

async fn do_genesis_ceremonial_one_validator(
	mcr_address: Address,
	anvil_address: &[mcr_settlement_config::anvil::AnvilAddressEntry],
	rpc_url: &str,
) -> Result<(), anyhow::Error> {
	//Define Signer. Signer1 is the MCRSettelement client
	let signer1: LocalWallet = anvil_address[0].private_key.parse()?;
	let signer1_addr: Address = anvil_address[0].address.parse()?;
	let signer1_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer1))
		.on_http(rpc_url.parse()?);
	let signer1_contract = MCR::new(mcr_address, &signer1_rpc_provider);

	stake_genesis(
		&signer1_rpc_provider,
		&signer1_contract,
		mcr_address,
		signer1_addr,
		55_000_000_000_000_000_000,
	)
	.await?;

	let signer2: LocalWallet = anvil_address[1].private_key.parse()?;
	let signer2_addr: Address = anvil_address[1].address.parse()?;
	let signer2_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer2))
		.on_http(rpc_url.parse()?);
	let signer2_contract = MCR::new(mcr_address, &signer2_rpc_provider);

	//init staking
	// Build a transaction to set the values.
	stake_genesis(
		&signer2_rpc_provider,
		&signer2_contract,
		mcr_address,
		signer2_addr,
		54_000_000_000_000_000_000,
	)
	.await?;

	let MCR::hasGenesisCeremonyEndedReturn { _0: has_genesis_ceremony_ended } =
		signer2_contract.hasGenesisCeremonyEnded().call().await?;
	let ceremony: bool = has_genesis_ceremony_ended.try_into().unwrap();
	assert!(ceremony);
	Ok(())
}
