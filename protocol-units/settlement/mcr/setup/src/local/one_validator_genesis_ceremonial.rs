use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use alloy_primitives::U256;
use anyhow::Context;
use mcr_settlement_client::eth_client::{MOVEToken, MovementStaking, MCR};
use mcr_settlement_config::Config;
use std::str::FromStr;
use tracing::info;

pub async fn setup(config: &Config) -> Result<(), anyhow::Error> {
	let rpc_url = config.eth_rpc_connection_url();

	let testing_config = config.testing.as_ref().context("Testing config not defined.")?;
	let deploy_config = config.deploy.as_ref().context("Deploy config not defined.")?;
	run_genesis_ceremony(
		&config,
		PrivateKeySigner::from_str(&deploy_config.mcr_deployment_account_private_key)?,
		&rpc_url,
		Address::from_str(&testing_config.move_token_contract_address)?,
		Address::from_str(&testing_config.movement_staking_contract_address)?,
		Address::from_str(&config.settle.mcr_contract_address)?,
	)
	.await
}

async fn run_genesis_ceremony(
	config: &Config,
	governor: PrivateKeySigner,
	rpc_url: &str,
	move_token_address: Address,
	staking_address: Address,
	mcr_address: Address,
) -> Result<(), anyhow::Error> {
	// Build validator client for MOVEToken, MCR, and staking
	// Validator is the e2e started node that we test.
	let validator: PrivateKeySigner = config.settle.signer_private_key.clone().parse()?;
	let validator_address = validator.address();
	tracing::info!("ICI Ceremony validator signer address:{validator_address}",);
	let validator_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(validator.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let validator_staking = MovementStaking::new(staking_address, &validator_rpc_provider);
	let validator_move_token = MOVEToken::new(move_token_address, &validator_rpc_provider);

	// Build bob client for MOVEToken, MCR, and staking
	// Bod act as another validator that we don't test.
	// It's to have at least 2 staking validator.
	let bob: PrivateKeySigner = config
		.testing
		.as_ref()
		.context("Testing config not defined.")?
		.well_known_account_private_keys
		.get(0)
		.context("No well known account")?
		.parse()?;
	let bob_address = bob.address();
	let bob_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(bob.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let bob_staking = MovementStaking::new(staking_address, &bob_rpc_provider);
	let bob_move_token = MOVEToken::new(move_token_address, &bob_rpc_provider);

	// Build MCR admin client to declare Validator and Bob
	let governor_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(governor.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let governor_token = MOVEToken::new(move_token_address, &governor_rpc_provider);
	let governor_mcr = MCR::new(mcr_address, &governor_rpc_provider);
	let governor_staking = MovementStaking::new(staking_address, &governor_rpc_provider);

	// Allow Validator and Bod to stake by adding to white list.
	governor_staking
		.whitelistAddress(validator_address)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to whilelist validator")?;
	governor_staking
		.whitelistAddress(bob_address)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to whilelist Bod")?;

	// alice stakes for mcr
	info!("Validator stakes for MCR");
	let token_name = governor_token.name().call().await.context("Failed to get token name")?;
	info!("Token name: {}", token_name._0);

	// debug: this is showing up correctly
	let has_minter_role = governor_token
		.hasMinterRole(governor.address())
		.call()
		.await
		.context("Failed to check if governor has minter role")?;
	info!("Governor Has minter role for governor: {}", has_minter_role._0);

	let has_minter_role_from_alice = validator_move_token
		.hasMinterRole(governor.address())
		.call()
		.await
		.context("Failed to check if governor has minter role")?;
	info!("Governoe Has minter role for Validator: {}", has_minter_role_from_alice._0);

	//info!("config chain_id: {}",config.eth_chain_id.clone().to_string());
	//info!("governor chain_id: {}", governor_rpc_provider.get_chain_id().await.context("Failed to get chain id")?.to_string());

	// debug: this is showing up correctly
	let alice_hash_minter_role = governor_token
		.hasMinterRole(validator_address)
		.call()
		.await
		.context("Failed to check if alice has minter role")?;
	info!("Validator has minter role for governor: {}", alice_hash_minter_role._0);

	// validator stakes for mcr
	governor_token
		.mint(validator_address, U256::from(100))
		//		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to mint for validator")?;
	validator_move_token
		.approve(staking_address, U256::from(95))
		.gas(5000000)
		.send()
		.await?
		.watch()
		.await
		.context("Validator failed to approve MCR")?;
	validator_staking
		.stake(mcr_address, move_token_address, U256::from(95))
		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Validator failed to stake for MCR")?;

	// bob stakes for mcr
	governor_token
		.mint(bob.address(), U256::from(100))
		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to mint for bob")?;
	bob_move_token
		.approve(staking_address, U256::from(5))
		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Bob failed to approve MCR")?;
	bob_staking
		.stake(mcr_address, move_token_address, U256::from(5))
		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Bob failed to stake for MCR")?;

	// mcr accepts the genesis
	info!("MCR accepts the genesis");
	governor_mcr
		.acceptGenesisCeremony()
		.gas(100000)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to accept genesis ceremony")?;
	info!("mcr accepted");

	Ok(())
}
