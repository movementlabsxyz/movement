use std::net::SocketAddr;

use super::test_helpers::start_rollup;
use borsh::BorshSerialize;
use jsonrpsee::core::client::{Subscription, SubscriptionClientT};
use jsonrpsee::rpc_params;
use m2_stf::genesis_config::GenesisPaths;
use m2_stf::RuntimeCall;
use sov_mock_da::{MockAddress, MockDaConfig, MockDaSpec};
use sov_modules_api::transaction::Transaction;
use sov_modules_api::{CryptoSpec, PrivateKey, Spec};
use sov_modules_stf_blueprint::kernels::basic::BasicKernelGenesisPaths;
use sov_sequencer::utils::SimpleClient;
use sov_stf_runner::RollupProverConfig;

const TOKEN_SALT: u64 = 0;
const TOKEN_NAME: &str = "test_token";

type TestSpec = sov_modules_api::default_spec::DefaultSpec<sov_mock_zkvm::MockZkVerifier>;
type DefaultPrivateKey = <<TestSpec as Spec>::CryptoSpec as CryptoSpec>::PrivateKey;

#[tokio::test]
async fn bank_tx_tests() -> Result<(), anyhow::Error> {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();

    let rollup_task = tokio::spawn(async {
        start_rollup(
            port_tx,
            GenesisPaths::from_dir("../../test-data/genesis/mock/"),
            BasicKernelGenesisPaths {
                chain_state: "../../test-data/genesis/mock/chain_state.json".into(),
            },
            RollupProverConfig::Skip,
            MockDaConfig {
                sender_address: MockAddress::new([0; 32]),
                finalization_blocks: 3,
                wait_attempts: 10,
            },
        )
        .await;
    });
    let port = port_rx.await.unwrap();

    // If the rollup throws an error, return it and stop trying to send the transaction
    tokio::select! {
        err = rollup_task => err?,
        res = send_test_create_token_tx(port) => res?,
    }
    Ok(())
}

async fn send_test_create_token_tx(rpc_address: SocketAddr) -> Result<(), anyhow::Error> {
    let key = DefaultPrivateKey::generate();
    let user_address: <TestSpec as Spec>::Address = key.to_address();

    let token_address =
        sov_bank::get_token_address::<TestSpec>(TOKEN_NAME, &user_address, TOKEN_SALT);

    let msg =
        RuntimeCall::<TestSpec, MockDaSpec>::bank(sov_bank::CallMessage::<TestSpec>::CreateToken {
            salt: TOKEN_SALT,
            token_name: TOKEN_NAME.to_string(),
            initial_balance: 1000,
            minter_address: user_address,
            authorized_minters: vec![],
        });
    let chain_id = 0;
    let gas_tip = 0;
    let gas_limit = 0;
    let nonce = 0;
    let max_gas_price = None;
    let tx = Transaction::<TestSpec>::new_signed_tx(
        &key,
        msg.try_to_vec().unwrap(),
        chain_id,
        gas_tip,
        gas_limit,
        max_gas_price,
        nonce,
    );

    let port = rpc_address.port();
    let client = SimpleClient::new("localhost", port).await?;

    let mut slot_processed_subscription: Subscription<u64> = client
        .ws()
        .subscribe(
            "ledger_subscribeSlots",
            rpc_params![],
            "ledger_unsubscribeSlots",
        )
        .await?;

    client.send_transactions(vec![tx], None).await.unwrap();
    // Wait until the rollup has processed the next slot
    let _ = slot_processed_subscription.next().await;

    let balance_response = sov_bank::BankRpcClient::<TestSpec>::balance_of(
        client.http(),
        None,
        user_address,
        token_address,
    )
    .await?;
    assert_eq!(balance_response.amount.unwrap_or_default(), 1000);
    Ok(())
}
