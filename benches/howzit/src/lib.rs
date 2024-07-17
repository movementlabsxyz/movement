pub mod howzit;

use std::path::PathBuf;

use aptos_types::transaction::TransactionPayload;
use aptos_framework::{
    BuiltPackage,
    BuildOptions
};
use aptos_sdk::{
    coin_client::{CoinClient, TransferOptions},
    rest_client::{Client, FaucetClient},
    transaction_builder::TransactionBuilder,
    /*types::{
        account_address::AccountAddress,
        transaction::{
            EntryFunction,

        }
    },
    move_types::{
        language_storage::{
            ModuleId,
            TypeTag
        },
        identifier::Identifier,
        
    },*/
    types::{chain_id::ChainId, LocalAccount},
};
use anyhow::Context;
use std::time::{SystemTime, UNIX_EPOCH};
// use std::str::FromStr;

pub struct PackagePublicationData {
    pub metadata_serialized: Vec<u8>,
    pub compiled_units: Vec<Vec<u8>>,
    pub payload: TransactionPayload,
}

pub async fn build_and_publish_package(
    wallet : LocalAccount,
    rest_client : Client,
    coin_client : CoinClient<'_>,
    faucet_client : FaucetClient,
    package_path : PathBuf,
    options : BuildOptions,
) -> Result<(), anyhow::Error> {

    // build the package
    let package = BuiltPackage::build(package_path, options)?;
    let compiled_units = package.extract_code();
    let metadata_serialized =
        bcs::to_bytes(&package.extract_metadata()?).expect("PackageMetadata has BCS");
    let payload = aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
        metadata_serialized.clone(),
        compiled_units.clone(),
    );

    // fund the account    
    faucet_client.fund(
        wallet.address(),
        1_000_000,
    ).await.context("Failed to fund account")?;

    // get the chain id
    let chain_id = rest_client
        .get_index()
        .await
        .context("Failed to get chain ID")?
        .inner()
        .chain_id;

    // build the publication transaction
    let transfer_options = TransferOptions::default();
    let transaction_builder = TransactionBuilder::new(
        payload,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + transfer_options.timeout_secs,
        ChainId::new(chain_id),
    )
    .sender(wallet.address())
    .sequence_number(wallet.sequence_number())
    .max_gas_amount(transfer_options.max_gas_amount)
    .gas_unit_price(transfer_options.gas_unit_price);
    let signed_txn = wallet.sign_with_transaction_builder(transaction_builder);


    let txn_hash = rest_client
        .submit(&signed_txn)
        .await
        .context("failed to submit publish transaction")?
        .into_inner();
    rest_client.wait_for_transaction(&txn_hash).await.context(
        "failed to wait for publish transaction",
    )?;


    Ok(())

}