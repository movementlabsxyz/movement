use std::{sync::Arc, time::Duration};
use anyhow::{Context, Result};
use ethers::{providers::Provider, utils::AnvilInstance};
use risc0_ethereum_relay::WalletKey;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const WAIT_DURATION: Duration = Duration::from_secs(5);
const MAX_RETRIES: u64 = 7 * 24 * 60 * 60 / WAIT_DURATION.as_secs(); // 1 week

/// Returns an empty Anvil builder. The default port is 8545. The mnemonic is
/// chosen randomly.
pub fn get_anvil() -> Option<AnvilInstance> {
    match std::env::var("ETHEREUM_HOST") {
        Ok(_) => None,
        _ => Some(ethers::utils::Anvil::new().spawn()),
    }
}

/// Returns a wallet key identifier defined by the env variable
/// `WALLET_KEY_IDENTIFIER` or from the given optional `anvil` instance.
pub fn get_wallet_key_identifier(anvil: Option<&AnvilInstance>) -> Result<WalletKey> {
    match std::env::var("WALLET_KEY_IDENTIFIER") {
        Ok(wallet_key_identifier) => wallet_key_identifier.try_into(),
        _ => {
            let anvil = anvil.context("Anvil not instantiated.")?;
            Ok(anvil.keys()[0].clone().into())
        }
    }
}

/// Returns an Ethereum Client Configuration struct.
pub async fn get_ethers_client_config(anvil: Option<&AnvilInstance>) -> Result<EthersClientConfig> {
    let provider = get_ws_provider(anvil).await.unwrap();
    let eth_node_url = get_ws_provider_endpoint(anvil).await.unwrap();
    let eth_chain_id = provider.get_chainid().await.unwrap().as_u64();
    let wallet_key_identifier = get_wallet_key_identifier(anvil).unwrap();
    let ethers_client_config = EthersClientConfig::new(
        eth_node_url,
        eth_chain_id,
        wallet_key_identifier,
        MAX_RETRIES,
        WAIT_DURATION,
    );
    Ok(ethers_client_config)
}

/// Returns an abstract provider for interacting with the Ethereum JSON RPC API
/// over Websockets.
pub async fn get_ws_provider(anvil: Option<&AnvilInstance>) -> Result<Provider<Ws>> {
    let endpoint = get_ws_provider_endpoint(anvil).await?;
    Ok(Provider::<Ws>::connect(&endpoint)
        .await
        .context("could not connect to {endpoint}")?
        .interval(POLL_INTERVAL))
}

/// Returns the Websocket endpoint for the Ethereum JSON RPC API.
pub async fn get_ws_provider_endpoint(anvil: Option<&AnvilInstance>) -> Result<String> {
    let endpoint = match std::env::var("ETHEREUM_HOST") {
        Ok(ethereum_host) => format!("ws://{ethereum_host}"),
        _ => anvil.context("Anvil not instantiated.")?.ws_endpoint(),
    };
    Ok(endpoint)
}
