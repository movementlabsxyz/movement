use anyhow::Result;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_sdk::rest_client::FaucetClient;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::move_utils::MemberId;
use aptos_sdk::types::transaction::EntryFunction;
use aptos_sdk::types::transaction::ExecutionStatus;
use aptos_sdk::types::transaction::SignedTransaction;
use aptos_sdk::types::transaction::TransactionInfo;
use aptos_sdk::types::transaction::TransactionInfoV0;
use aptos_sdk::{
    rest_client::Client,
    transaction_builder::TransactionFactory,
    types::{
//        account_address::AccountAddress,
        chain_id::ChainId, //SignedTransaction, TransactionArgument
        LocalAccount,
    },
};
use buildtime_helpers::cargo::cargo_workspace;
use std::path::PathBuf;



// static SCRIPT: &[u8] =
//     include_bytes!("move/build/malicious/bytecode_scripts/register.mv");

const MOVE_SCRIPT_PATH: &str = "networks/suzuka/suzuka-client/src/bin/malicious/move";
const CHAIN_ID: u8 = 4;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let dot_movement = dot_movement::DotMovement::try_from_env()?;
    let suzuka_config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;
    let rpc_url = suzuka_config.execution_config.maptos_config.client.get_rest_url()?;
    let faucet_url = suzuka_config.execution_config.maptos_config.client.get_faucet_url()?;

    let chain_id = ChainId::new(CHAIN_ID);
    let client = Client::new(rpc_url.clone());


    //set execution path
    let root: PathBuf = cargo_workspace()?;
    let additional_path = MOVE_SCRIPT_PATH;
    let move_exe_path = root.join(additional_path);

    //init aptos only done one time
    let init_output =
        commander::run_command_current_dir("/bin/bash", &["aptos init --network custom --rest-url {node_url} --faucet-url {faucet_url} --assume-yes"], Some(&move_exe_path)).await?;
    println!("{}", init_output);

    //Init Alice and Bob accounts
    let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let alice_address = alice.address();
    let faucet_client = FaucetClient::new(faucet_url, rpc_url);
    faucet_client.fund(alice.address(), 100_000_000).await?;

    //Publish MoinCoin with Alice account
    let package_path = move_exe_path.to_string_lossy();
    println!("exe_path:{}", package_path);
    let alice_private_key = alice.private_key().to_encoded_string()?;

    let publish_cmd = format!("aptos move publish --private-key {alice_private_key} --sender-account {alice_address} --package-dir {package_path} --named-addresses malicious_test={alice_address} --assume-yes");
    println!("{}", publish_cmd);
    let publish_output =
        commander::run_command("/bin/bash", &[&publish_cmd]).await?;
    println!("{}", publish_output);

    let _ = tokio::time::sleep(tokio::time::Duration::from_millis(5000));

    let mut alice_caller =  FunctionCaller::build(alice, client.clone(), chain_id).await?;
    //Alice Mint token
     let tx_result = alice_caller.run_function(
        &format!("0x1::managed_coin::mint"), 
        vec![moncoin_tytag(alice_address)],
        vec![bcs::to_bytes(&alice_address).unwrap(), bcs::to_bytes(&(20000000000000000 as u64)).unwrap()]
    )
    .await?;
    println!("RESULTTTTTTT Mint tx_receipt_data: {tx_result:?}",);

    //Init Bob account
    let bob = LocalAccount::generate(&mut rand::rngs::OsRng);
    let bob_address = bob.address();
    faucet_client.fund(bob.address(), 100_000_000).await?;
    let mut bob_caller =  FunctionCaller::build(bob, client.clone(), chain_id).await?;


    //Bob try to tranfer some USD to Alice. Generate INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE error
    let tx_result = bob_caller.run_function("0x1::aptos_account::transfer_coins", vec![moncoin_tytag(alice_address)], vec![
            bcs::to_bytes(&alice_address).unwrap(),
            bcs::to_bytes(&(1000 as u64)).unwrap(),
        ]).await;
    println!("RESULTTTTTTT Bob bad transfer result: {tx_result:?}",);
    assert!(
        tx_result.is_err(),
        "Bob INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE allowed."
    );

    //Alice Transfer USDT to Bod with wrong signer
    let tx_result = bob_caller.run_function_with_signer(&alice_caller.account, "0x1::aptos_account::transfer_coins", vec![moncoin_tytag(alice_address)], vec![
            bcs::to_bytes(&bob_address).unwrap(),
            bcs::to_bytes(&(1000 as u64)).unwrap(),
        ]).await;
    println!("RESULTTTTTTT Bob bad transfer result: {tx_result:?}",);
    assert!(
        tx_result.is_err(),
        "Bob INVALID_AUTH_KEY allowed."
    );

    //Transfer to Alice with the wrong arguments
    let tx_result = bob_caller.run_function("0x1::aptos_account::transfer_coins", vec![moncoin_tytag(alice_address)], vec![
            bcs::to_bytes(&alice_address).unwrap(),
        ]).await;
    println!("RESULTTTTTTT Bob bad transfer result: {tx_result:?}",);
    assert!(
        tx_result.is_err(),
        "WRONG ARGUMENTS allowed."
    );




    Ok(())
}

fn moncoin_tytag(account_address: AccountAddress) -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: account_address,
        module: Identifier::new("moon_coin").unwrap(),
        name: Identifier::new("MoonCoin").unwrap(),
        type_params: vec![],
    }))
}

struct FunctionCaller {
    pub account: LocalAccount,
    client: Client,
    sequence_number: u64,
    chainid: ChainId,
}

impl FunctionCaller {
    async fn build(account: LocalAccount, client: Client, chainid: ChainId) -> Result<Self> {
        let account_rpc = client.get_account(account.address()).await?;
        let sequence_number = account_rpc.inner().sequence_number;
        Ok(FunctionCaller {
            account,
            client,
            sequence_number,
            chainid,
        })
    }

//     async fn run_script(&mut self, code: Vec<u8>,
//     ty_args: Vec<TypeTag>,
//     args: Vec<TransactionArgument>) -> Result<TransactionInfoV0> {
//         let transaction_factory = TransactionFactory::new(self.chainid);
//         let raw_tx = transaction_factory.script(Script::new(
//             code,
//             ty_args,
//             args,
//         ))
//             .sender(self.account.address())
//             .sequence_number(self.sequence_number).build();
//         let signed_transaction = self.account.sign_transaction(raw_tx);

// //    println!("signed_transaction:{signed_transaction:?}", );
//         let res = self._submit_signed_tx(&signed_transaction).await?;
//         self.sequence_number +=1;
//         Ok(res)
//     }

    async fn run_function(
        &mut self,
        function_id: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<TransactionInfoV0> {

        let res = self._exec_tx(&self.account, &self.account, function_id, ty_args, args).await?;
        self.sequence_number +=1;
        Ok(res)

    }

    async fn run_function_with_signer(
        &mut self,
        signer: &LocalAccount,
        function_id: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<TransactionInfoV0> {
        let res = self._exec_tx(&self.account, signer, function_id, ty_args, args).await?;
        self.sequence_number +=1;
        Ok(res)
    }

    async fn _exec_tx(&self,
        account: &LocalAccount,
        signer: &LocalAccount,
        function_id: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,) -> Result<TransactionInfoV0> {

        let MemberId {
            module_id,
            member_id,
        } = str::parse(function_id)?;

        let entry_function = EntryFunction::new(module_id, member_id, ty_args, args);
        let raw_tx = TransactionFactory::new(self.chainid)
            .entry_function(entry_function)
            .sender(account.address())
            .sequence_number(self.sequence_number)
            .build();
        //    println!("raw_tx:{raw_tx:?}",);

        let signed_transaction = signer.sign_transaction(raw_tx);
        
        let res = self._submit_signed_tx(&signed_transaction).await?;
        Ok(res)

    }

    async fn _submit_signed_tx(&self, signed_transaction: &SignedTransaction)-> Result<TransactionInfoV0> {
        let pending_txn = self.client.submit(signed_transaction).await?.into_inner();
        let tx_receipt_data = self.client.wait_for_transaction_bcs(&pending_txn).await?;
        //    println!("RESULTTTTTTT run_function: {tx_receipt_data:?}",);

        let TransactionInfo::V0(tx_info) = tx_receipt_data.into_inner().info;

        if let ExecutionStatus::Success = tx_info.status() {
            Ok(tx_info)
        } else {
            println!("Tx fail with result {tx_info:?}",);
            Err(anyhow::anyhow!(format!("Tx send fail:{tx_info:?}")).into())
        }
    }

}