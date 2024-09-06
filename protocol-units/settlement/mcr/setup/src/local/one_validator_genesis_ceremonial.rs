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
	// Build alice client for MOVEToken, MCR, and staking
	info!("Creating alice client");
	// let alice: PrivateKeySigner = config
	// 	.testing
	// 	.as_ref()
	// 	.context("Testing config not defined.")?
	// 	.well_known_account_private_keys
	// 	.get(1)
	// 	.context("No well known account")?
	// 	.parse()?;
	let alice: PrivateKeySigner = config.settle.signer_private_key.clone().parse()?;
	let alice_address = alice.address();
	let alice_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(alice.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let alice_staking = MovementStaking::new(staking_address, &alice_rpc_provider);
	let alice_move_token = MOVEToken::new(move_token_address, &alice_rpc_provider);

	// Build bob client for MOVEToken, MCR, and staking
	info!("Creating bob client");
	let bob: PrivateKeySigner = config
		.testing
		.as_ref()
		.context("Testing config not defined.")?
		.well_known_account_private_keys
		.get(2)
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

	// Build the MCR client for staking
	info!("Creating governor client");
	let governor_rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(governor.clone()))
		.on_builtin(&rpc_url)
		.await?;
	let governor_token = MOVEToken::new(move_token_address, &governor_rpc_provider);
	let governor_mcr = MCR::new(mcr_address, &governor_rpc_provider);
	let governor_staking = MovementStaking::new(staking_address, &governor_rpc_provider);

	// Allow Alice and Bod to stake by adding to white list.
	governor_staking
		.whitelistAddress(alice_address)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to whilelist alice")?;
	governor_staking
		.whitelistAddress(bob_address)
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to whilelist Bod")?;

	// alice stakes for mcr
	info!("Alice stakes for MCR");
	let token_name = governor_token.name().call().await.context("Failed to get token name")?;
	info!("Token name: {}", token_name._0);

	// debug: this is showing up correctly
	let has_minter_role = governor_token
		.hasMinterRole(governor.address())
		.call()
		.await
		.context("Failed to check if governor has minter role")?;
	info!("Has minter role: {}", has_minter_role._0);

	let has_minter_role_from_alice = alice_move_token
		.hasMinterRole(governor.address())
		.call()
		.await
		.context("Failed to check if governor has minter role")?;
	info!("Has minter role from Alice: {}", has_minter_role_from_alice._0);

	//info!("config chain_id: {}",config.eth_chain_id.clone().to_string());
	//info!("governor chain_id: {}", governor_rpc_provider.get_chain_id().await.context("Failed to get chain id")?.to_string());

	// debug: this is showing up correctly
	let alice_hash_minter_role = governor_token
		.hasMinterRole(alice.address())
		.call()
		.await
		.context("Failed to check if alice has minter role")?;
	info!("Alice has minter role: {}", alice_hash_minter_role._0);

	let governor_address = governor.address();
	info!("Governor address: {}", governor_address.clone().to_string());
	// debug: fails here
	governor_token
		.mint(alice_address, U256::from(100))
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to mint for alice")?;

	info!("staking_address: {}", staking_address.clone().to_string());

	// debug: also fails here if you lift the restriction above; then it fails as if msg.sender =  address(0)
	alice_move_token
		.approve(staking_address, U256::from(100))
		.send()
		.await?
		.watch()
		.await
		.context("Alice failed to approve MCR")?;
	info!("Alice move approve");
	let callbuilder = alice_staking.stake(mcr_address, move_token_address, U256::from(100));
	let rc = callbuilder.send().await;
	match rc {
		Ok(rc) => {
			rc.watch().await.context("Alice failed to stake for MCR")?;
		}
		Err(err) => {
			println!("err: {err:?}");
			panic!("{err:?}",);
		}
	};
	info!("Alice move staking");

	// bob stakes for mcr
	info!("Bob stakes for MCR");
	governor_token
		.mint(bob.address(), U256::from(100))
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to mint for bob")?;
	info!("governor mint");

	let bob_balance = bob_move_token
		.balanceOf(bob.address())
		.call()
		.await
		.context("Failed to get bob balance")?;
	info!("Bob balance: {}", bob_balance._0);
	bob_move_token
		.approve(staking_address, U256::from(100))
		.send()
		.await?
		.watch()
		.await
		.context("Bob failed to approve MCR")?;
	info!("Bob move approve");
	bob_staking
		.stake(mcr_address, move_token_address, U256::from(100))
		.send()
		.await?
		.watch()
		.await
		.context("Bob failed to stake for MCR")?;
	info!("Bob move staking");

	// let domain_time = governor_staking
	// .epochDurationByDomain(mcr_address.clone())
	// .call()
	// .await.context("Failed to get domain registration time")?;
	// info!("Domain registration time in MCR {:?}", domain_time);
	// mcr accepts the genesis
	info!("MCR accepts the genesis");
	governor_mcr
		.acceptGenesisCeremony()
		.send()
		.await?
		.watch()
		.await
		.context("Governor failed to accept genesis ceremony")?;
	info!("mcr accepted");

	// send Height 1 commitment that is not send by the Suzuka node
	let height0_commitment = MCR::BlockCommitment {
		height: U256::from(1),
		commitment: alloy_primitives::FixedBytes([1; 32].try_into()?),
		blockId: alloy_primitives::FixedBytes([2; 32].try_into()?),
	};
	let validator_mcr = MCR::new(mcr_address, &alice_rpc_provider);
	let call_builder = validator_mcr.submitBlockCommitment(height0_commitment.clone());
	let call_builder = call_builder.clone().gas(3_000_000);
	let res = call_builder.send().await?.get_receipt().await?;

	let bob_mcr = MCR::new(mcr_address, &bob_rpc_provider);
	let call_builder = bob_mcr.submitBlockCommitment(height0_commitment);
	let call_builder = call_builder.clone().gas(3_000_000);
	let res = call_builder.send().await?.get_receipt().await?;

	// Try to get an accepted commitment
	let MCR::getAcceptedCommitmentAtBlockHeightReturn {
		_0: get_accepted_commitment_at_block_height,
	} = validator_mcr.getAcceptedCommitmentAtBlockHeight(U256::from(1)).call().await?;

	info!(
		"get_accepted_commitment_at_block_height 1 commitment:{:?} blockId:{:?}",
		get_accepted_commitment_at_block_height.commitment,
		get_accepted_commitment_at_block_height.blockId,
	);

	info!("Commitment height 1 send");

	Ok(())
}

// async fn run_genesis_ceremony(
// 	config: &Config,
// 	governor: PrivateKeySigner,
// 	rpc_url: &str,
// 	move_token_address: Address,
// 	staking_address: Address,
// 	mcr_address: Address,
// ) -> Result<(), anyhow::Error> {
// 	// Build validator client for MOVEToken, MCR, and staking
// 	// Validator is the e2e started node that we test.
// 	let validator: PrivateKeySigner = config.settle.signer_private_key.clone().parse()?;
// 	let validator_address = validator.address();
// 	tracing::info!("ICI Ceremony validator signer address:{validator_address}",);
// 	let validator_rpc_provider = ProviderBuilder::new()
// 		.with_recommended_fillers()
// 		.wallet(EthereumWallet::from(validator.clone()))
// 		.on_builtin(&rpc_url)
// 		.await?;
// 	let validator_staking = MovementStaking::new(staking_address, &validator_rpc_provider);
// 	let validator_move_token = MOVEToken::new(move_token_address, &validator_rpc_provider);

// 	// Build bob client for MOVEToken, MCR, and staking
// 	// Bod act as another validator that we don't test.
// 	// It's to have at least 2 staking validator.
// 	let bob: PrivateKeySigner = config
// 		.testing
// 		.as_ref()
// 		.context("Testing config not defined.")?
// 		.well_known_account_private_keys
// 		.get(0)
// 		.context("No well known account")?
// 		.parse()?;
// 	let bob_address = bob.address();
// 	let bob_rpc_provider = ProviderBuilder::new()
// 		.with_recommended_fillers()
// 		.wallet(EthereumWallet::from(bob.clone()))
// 		.on_builtin(&rpc_url)
// 		.await?;
// 	let bob_staking = MovementStaking::new(staking_address, &bob_rpc_provider);
// 	let bob_move_token = MOVEToken::new(move_token_address, &bob_rpc_provider);

// 	// Build MCR admin client to declare Validator and Bob
// 	let governor_rpc_provider = ProviderBuilder::new()
// 		.with_recommended_fillers()
// 		.wallet(EthereumWallet::from(governor.clone()))
// 		.on_builtin(&rpc_url)
// 		.await?;
// 	let governor_token = MOVEToken::new(move_token_address, &governor_rpc_provider);
// 	let governor_mcr = MCR::new(mcr_address, &governor_rpc_provider);
// 	let governor_staking = MovementStaking::new(staking_address, &governor_rpc_provider);

// 	// Allow Validator and Bod to stake by adding to white list.
// 	governor_staking
// 		.whitelistAddress(validator_address)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Governor failed to whilelist validator")?;
// 	governor_staking
// 		.whitelistAddress(bob_address)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Governor failed to whilelist Bod")?;

// 	// alice stakes for mcr
// 	info!("Validator stakes for MCR");
// 	let token_name = governor_token.name().call().await.context("Failed to get token name")?;
// 	info!("Token name: {}", token_name._0);

// 	// debug: this is showing up correctly
// 	let has_minter_role = governor_token
// 		.hasMinterRole(governor.address())
// 		.call()
// 		.await
// 		.context("Failed to check if governor has minter role")?;
// 	info!("Governor Has minter role for governor: {}", has_minter_role._0);

// 	let has_minter_role_from_alice = validator_move_token
// 		.hasMinterRole(governor.address())
// 		.call()
// 		.await
// 		.context("Failed to check if governor has minter role")?;
// 	info!("Governoe Has minter role for Validator: {}", has_minter_role_from_alice._0);

// 	//info!("config chain_id: {}",config.eth_chain_id.clone().to_string());
// 	//info!("governor chain_id: {}", governor_rpc_provider.get_chain_id().await.context("Failed to get chain id")?.to_string());

// 	// debug: this is showing up correctly
// 	let alice_hash_minter_role = governor_token
// 		.hasMinterRole(validator_address)
// 		.call()
// 		.await
// 		.context("Failed to check if alice has minter role")?;
// 	info!("Validator has minter role for governor: {}", alice_hash_minter_role._0);

// 	// validator stakes for mcr
// 	governor_token
// 		.mint(validator_address, U256::from(100))
// 		//		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Governor failed to mint for validator")?;
// 	validator_move_token
// 		.approve(staking_address, U256::from(95))
// 		.gas(5000000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Validator failed to approve MCR")?;
// 	validator_staking
// 		.stake(mcr_address, move_token_address, U256::from(95))
// 		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Validator failed to stake for MCR")?;

// 	// bob stakes for mcr
// 	governor_token
// 		.mint(bob.address(), U256::from(100))
// 		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Governor failed to mint for bob")?;
// 	bob_move_token
// 		.approve(staking_address, U256::from(5))
// 		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Bob failed to approve MCR")?;
// 	bob_staking
// 		.stake(mcr_address, move_token_address, U256::from(5))
// 		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Bob failed to stake for MCR")?;

// 	// mcr accepts the genesis
// 	info!("MCR accepts the genesis");
// 	governor_mcr
// 		.acceptGenesisCeremony()
// 		.gas(100000)
// 		.send()
// 		.await?
// 		.watch()
// 		.await
// 		.context("Governor failed to accept genesis ceremony")?;
// 	info!("mcr accepted");

// 	Ok(())
// }
