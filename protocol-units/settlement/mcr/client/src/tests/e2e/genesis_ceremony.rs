use mcr_settlement_client::eth_client::{
    MCR,
    MOVEToken,
    MovementStaking
};
use mcr_settlement_config::Config;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_primitives::Address;
use alloy_primitives::U256;

use godfig::{
    Godfig,
    backend::config_file::ConfigFile
};


async fn run_genesis_ceremony(
    governor: LocalWallet,
    rpc_url: &str,
    mcr_address: Address,
    staking_address: Address,
    move_token_address: Address,
) -> Result<(), anyhow::Error> {

    // Build alice client for MOVEToken, MCR, and staking
    let alice = LocalWallet::random();
    let alice_rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .signer(EthereumSigner::from(alice))
        .on_http(rpc_url.parse()?);
    let alice_mcr = MCR::new(mcr_address, &alice_rpc_provider);
    let alice_staking = MovementStaking::new(staking_address, &alice_rpc_provider);
    let alice_move_token = MOVEToken::new(move_token_address, &alice_rpc_provider);

    // Build bob client for MOVEToken, MCR, and staking
    let bob: LocalWallet = LocalWallet::random();
    let bob_rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .signer(EthereumSigner::from(bob))
        .on_http(rpc_url.parse()?);
    let bob_mcr = MCR::new(mcr_address, &bob_rpc_provider);
    let bob_staking = MovementStaking::new(staking_address, &bob_rpc_provider);
    let bob_move_token = MOVEToken::new(move_token_address, &bob_rpc_provider);

    // Build the MCR client for staking
    let governor_rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .signer(EthereumSigner::from(governor_signer))
        .on_http(rpc_url.parse()?);
    let governor_token = MOVEToken::new(move_token_address, &governor_rpc_provider);
    let governor_staking = MovementStaking::new(staking_address, &governor_rpc_provider);

    // alice stakes for mcr
    governor_token
        .mint(alice.address(), U256::from(100))
        .call()
        .await?;
    alice_move_token.approve(mcr_address, U256::from(100)).call().await?;
    alice_staking
        .stake(mcr_address, move_token_address, U256::from(100))
        .call()
        .await?;

    // bob stakes for mcr
    governor_token
        .mint(bob.address(), U256::from(100))
        .call()
        .await?;
    bob_move_token.approve(mcr_address, U256::from(100)).call().await?;
    bob_staking
        .stake(mcr_address, move_token_address, U256::from(100))
        .call()
        .await?;

    // mcr accepts the genesis
    governor_staking.acceptGenesisCeremony().call().await?;

    Ok(())
}

#[tokio::test]
pub fn test_genesis_ceremony() -> Result<(), anyhow::Error> {
    
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

    Ok(())
}

