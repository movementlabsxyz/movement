use anyhow::Context;
use aptos_sdk::{
    crypto::ValidCryptoMaterialStringExt,
    rest_client::{Client, FaucetClient},
    transaction_builder::TransactionBuilder,
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{Script, TransactionPayload},
    },
};
use movement_client::{
    crypto::ed25519::Ed25519PrivateKey,
    types::LocalAccount,
    coin_client::CoinClient,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};
use url::Url;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
    let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
    dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap()
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
    let node_connection_address = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_rest_connection_hostname
        .clone();
    let node_connection_port = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_rest_connection_port;
    let node_connection_url =
        format!("http://{}:{}", node_connection_address, node_connection_port);
    Url::from_str(&node_connection_url).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
    let faucet_listen_address = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_faucet_rest_connection_hostname
        .clone();
    let faucet_listen_port = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_faucet_rest_connection_port;
    let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
    Url::from_str(&faucet_listen_url).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Initialize clients
    let rest_client = Client::new(NODE_URL.clone());
    let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
    let coin_client = CoinClient::new(&rest_client);

    // Load core resource account
    let raw_private_key = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .chain
        .maptos_private_key_signer_identifier
        .try_raw_private_key()?;
    let private_key = Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;
    let mut core_resources_account =
        LocalAccount::from_private_key(private_key.to_encoded_string()?.as_str(), 0)?;

    tracing::info!("Core Resources Account address: {}", core_resources_account.address());

    // Fund the account
    faucet_client
        .fund(core_resources_account.address(), 100_000_000_000)
        .await
        .context("Failed to fund core resources account")?;

    // Get initial balance
    let initial_balance = coin_client
        .get_account_balance(&core_resources_account.address())
        .await
        .context("Failed to get initial balance")?;

    tracing::info!("Initial core resources balance: {}", initial_balance);

    // Compile and get the bytecode for our script
    // Note: In practice, you'd compile this ahead of time and load the bytecode
    let script_bytecode = include_bytes!("../move/scripts/remove_privileged_resources.mv");
    
    // Create and submit transaction with the script
    let payload = TransactionPayload::Script(Script::new(
        script_bytecode.to_vec(),
        vec![], // No type arguments
        vec![], // No arguments needed since the address is hardcoded in the script
    ));

    let transaction = core_resources_account.sign_with_transaction_builder(
        TransactionBuilder::new(payload, 0, ChainId::test())
            .sender(core_resources_account.address())
            .sequence_number(core_resources_account.sequence_number())
            .max_gas_amount(2000)
            .gas_unit_price(100)
            .expiration_timestamp_secs(u64::MAX),
    );

    // Submit transaction
    let txn_hash = rest_client
        .submit(&transaction)
        .await
        .context("Failed to submit transaction")?;

    tracing::info!("Transaction submitted: {:?}", txn_hash);

    // Wait for transaction completion
    rest_client
        .wait_for_transaction(&txn_hash.into_inner())
        .await
        .context("Failed waiting for transaction")?;

    tracing::info!("Transaction completed successfully");

    // Verify resources are removed
    let account_state = rest_client
        .get_account_resources(AccountAddress::from_hex_literal("0xA550C18").unwrap())
        .await
        .context("Failed to get account resources")?;

    let resources = account_state.into_inner();
    
    // Verify DelegatedMintCapability is removed
    assert!(
        !resources.iter().any(|r| r.resource_type.to_string().contains("DelegatedMintCapability")),
        "DelegatedMintCapability still exists"
    );

    // Verify SetVersionCapability is removed
    assert!(
        !resources.iter().any(|r| r.resource_type.to_string().contains("SetVersionCapability")),
        "SetVersionCapability still exists"
    );

    tracing::info!("Successfully verified resources were removed");

    Ok(())
}