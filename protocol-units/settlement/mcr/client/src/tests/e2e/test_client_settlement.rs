use crate::eth_client::{MOVEToken, MovementStaking, MCR};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use alloy_primitives::U256;
use mcr_settlement_config::Config;
use std::str::FromStr;

use anyhow::Context;
use godfig::{backend::config_file::ConfigFile, Godfig};
use tracing::info;
// use alloy::rpc::types::trace::parity::TraceType;
// use alloy_rpc_types::TransactionRequest;

async fn run_genesis_ceremony(
    config : &Config,
    governor: PrivateKeySigner,
    rpc_url: &str,
    move_token_address: Address,
    staking_address: Address,
    mcr_address: Address,
) -> Result<(), anyhow::Error> {

    // Build alice client for MOVEToken, MCR, and staking
    info!("Creating alice client");
    let alice : PrivateKeySigner = config.well_known_accounts.get(1).context("No well known account")?.parse()?;
    let alice_address : Address = config.well_known_addresses.get(1).context("No well known address")?.parse()?;
    let alice_rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(alice.clone()))
        .on_builtin(&rpc_url)
        .await?;
    let alice_staking = MovementStaking::new(staking_address, &alice_rpc_provider);
    let alice_move_token = MOVEToken::new(move_token_address, &alice_rpc_provider);

    // Build bob client for MOVEToken, MCR, and staking
    info!("Creating bob client");
    let bob: PrivateKeySigner = config.well_known_accounts.get(2).context("No well known account")?.parse()?;
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

    // alice stakes for mcr
    info!("Alice stakes for MCR");
    let token_name = governor_token.
        name().call().await.context("Failed to get token name")?;
    info!("Token name: {}", token_name._0);

    // debug: this is showing up correctly
    let has_minter_role = governor_token
        .hasMinterRole(governor.address())
         
        .call().await
        .context("Failed to check if governor has minter role")?;
    info!("Has minter role: {}", has_minter_role._0);

    let has_minter_role_from_alice = alice_move_token
        .hasMinterRole(governor.address()) 
        .call().await
        .context("Failed to check if governor has minter role")?;
    info!("Has minter role from Alice: {}", has_minter_role_from_alice._0);


    //info!("config chain_id: {}",config.eth_chain_id.clone().to_string());
    //info!("governor chain_id: {}", governor_rpc_provider.get_chain_id().await.context("Failed to get chain id")?.to_string());

    // debug: this is showing up correctly
    let alice_hash_minter_role = governor_token
        .hasMinterRole(alice.address()) 
        .call().await
        .context("Failed to check if alice has minter role")?;
    info!("Alice has minter role: {}", alice_hash_minter_role._0);

    let governor_address = governor.address();
    info!("Governor address: {}", governor_address.clone().to_string());
    // debug: fails here
    governor_token
        .mint(alice_address, U256::from(100))
        .send().await?.watch().await.context("Governor failed to mint for alice")?;

    info!("staking_address: {}", staking_address.clone().to_string());

    // debug: also fails here if you lift the restriction above; then it fails as if msg.sender =  address(0)
    alice_move_token
        .approve(staking_address, U256::from(100))
        .send().await?.watch().await.context("Alice failed to approve MCR")?;
    info!("Alice move approve");
    alice_staking
        .stake(mcr_address, move_token_address, U256::from(100))
        .send().await?.watch().await.context("Alice failed to stake for MCR")?;
    info!("Alice move staking");

    // bob stakes for mcr
    info!("Bob stakes for MCR");
    governor_token
        .mint(bob.address(), U256::from(100))
        .send().await?.watch().await.context("Governor failed to mint for bob")?;
    info!("governor mint");

    let bob_balance = bob_move_token
        .balanceOf(bob.address())
        .call().await.context("Failed to get bob balance")?;
    info!("Bob balance: {}", bob_balance._0);
    bob_move_token
        .approve(staking_address, U256::from(100))
        .send().await?.watch().await.context("Bob failed to approve MCR")?;
    info!("Bob move approve");
    bob_staking
        .stake(mcr_address, move_token_address, U256::from(100))
        .send().await?.watch().await.context("Bob failed to stake for MCR")?;
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
        .send().await?.watch().await.context("Governor failed to accept genesis ceremony")?;
    info!("mcr accepted");

    Ok(())
}

use crate::eth_client::Client;
use crate::McrSettlementClientOperations;
use movement_types::Commitment;
use movement_types::Id;
use movement_types::BlockCommitment;
use tokio_stream::StreamExt;

#[tokio::test]
pub async fn test_client_settlement() -> Result<(), anyhow::Error> {

    use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();
    
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![
        "mcr_settlement".to_string(),
    ]);
    let config : Config = godfig.try_wait_for_ready().await?;
    let rpc_url = config.eth_rpc_connection_url();


    run_genesis_ceremony(
        &config,
        PrivateKeySigner::from_str(&config.governor_private_key)?,
        &rpc_url,
        Address::from_str(&config.move_token_contract_address)?,
        Address::from_str(&config.movement_staking_contract_address)?,
        Address::from_str(&config.mcr_contract_address)?
    ).await?;

    // Build client 1 and send the first commitment.
    let config1 = Config {
        signer_private_key: config.well_known_accounts.get(1).context("No well known account")?.to_string(),
        ..config.clone()
    };
    let client1 = Client::build_with_config(config1).await.unwrap();

    let mut client1_stream = client1.stream_block_commitments().await.unwrap();
    // Client post a new commitment
    let commitment =
        BlockCommitment { height: 1, block_id: Id([2; 32]), commitment: Commitment([3; 32]) };

    let res = client1.post_block_commitment(commitment.clone()).await;
    assert!(res.is_ok());

    // No notification, quorum is not reached
    let res =
        tokio::time::timeout(tokio::time::Duration::from_secs(5), client1_stream.next()).await;
    assert!(res.is_err());

    // Build client 2 and send the second commitment.
    let config2 = Config {
        signer_private_key: config.well_known_accounts.get(2).context("No well known account")?.to_string(),
        ..config.clone()
    };
    let client2 = Client::build_with_config(config2).await.unwrap();

    let mut client2_stream = client2.stream_block_commitments().await.unwrap();

    // Client post a new commitment
    let res = client2.post_block_commitment(commitment).await;    assert!(res.is_ok());

    // Now we move to block 2 and make some commitment just to trigger the epochRollover
    let commitment2 =
        BlockCommitment { height: 2, block_id: Id([4; 32]), commitment: Commitment([5; 32]) };

    let res = client2.post_block_commitment(commitment2.clone()).await;
    assert!(res.is_ok());

    // Validate that the accepted commitment stream gets the event.
    let event =
        tokio::time::timeout(tokio::time::Duration::from_secs(7), client1_stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    assert_eq!(event.commitment.0[0], 3);
    assert_eq!(event.block_id.0[0], 2);

    let event =
        tokio::time::timeout(tokio::time::Duration::from_secs(7), client2_stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    assert_eq!(event.commitment.0[0], 3);
    assert_eq!(event.block_id.0[0], 2);

    // Test post batch commitment
    // Post the complementary batch on height 2 and one on height 3
    let commitment3 =
        BlockCommitment { height: 3, block_id: Id([6; 32]), commitment: Commitment([7; 32]) };
    let res = client1.post_block_commitment_batch(vec![commitment2, commitment3]).await;
    assert!(res.is_ok());
    // Validate that the commitments stream gets the event.
    let event =
        tokio::time::timeout(tokio::time::Duration::from_secs(5), client1_stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    assert_eq!(event.commitment.0[0], 5);
    assert_eq!(event.block_id.0[0], 4);
    let event =
        tokio::time::timeout(tokio::time::Duration::from_secs(7), client2_stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    assert_eq!(event.commitment.0[0], 5);
    assert_eq!(event.block_id.0[0], 4);

    // Test get_commitment_at_height
    let commitment = client1.get_commitment_at_height(1).await?;
    assert!(commitment.is_some());
    let commitment = commitment.unwrap();
    assert_eq!(commitment.commitment.0[0], 3);
    assert_eq!(commitment.block_id.0[0], 2);
    let commitment = client1.get_commitment_at_height(10).await?;
    assert_eq!(commitment, None);

    Ok(())
}

