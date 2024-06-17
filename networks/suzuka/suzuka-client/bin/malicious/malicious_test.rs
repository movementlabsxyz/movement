use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::move_types::ident_str;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_sdk::rest_client::FaucetClient;
use aptos_sdk::transaction_builder::TransactionBuilder;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::move_utils::MemberId;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use aptos_sdk::types::transaction::EntryFunction;
use aptos_sdk::types::transaction::TransactionPayload;
use aptos_sdk::{
    rest_client::Client,
    transaction_builder::TransactionFactory,
    types::{
//        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{Script, }, //SignedTransaction, TransactionArgument
        LocalAccount,
    },
};


static SCRIPT: &[u8] =
    include_bytes!("move/build/malicious/bytecode_scripts/register.mv");

const PRIVATE_KEY: &str = "0xcb1fe7df72aff4a114d2bff60ecce2172f342b66cd5dafb2b8844b25e29b8d58";
//const PUBLIC_KEY: &str = "0xd82405d9faa256840ff6a8fe78d28d3f43581b1d34aa7f78476f4ce7e47a9e92";
const CHAIN_ID: u8 = 4;
//const RPC_URL: &str = "http://127.0.0.1:30731";
const RPC_URL: &str = "http://127.0.0.1:8080";
const FAUCET_URL: &str = "http://127.0.0.1:30732";

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    //Init Alice and Bob accounts
    let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let alice_address = alice.address();
    faucet_client.fund(alice.address(), 100_000_000).await?;
    let mut alice_caller =  FunctionCaller::build(alice, client.clone(), chain_id).await?;

    //Register MonCoin for Alice
    let tx_result = alice_caller.run_script(
        SCRIPT.to_vec(),
        vec![],
        vec![],
    )
    .await?;
    println!("RESULTTTTTTT initialize: {tx_result:?}",);

    // Build a transaction factory.
    let transaction_factory = TransactionFactory::new(ChainId::new(CHAIN_ID));

    let private_key = Ed25519PrivateKey::from_encoded_string(
        PRIVATE_KEY,
    )?;
    let public_key = Ed25519PublicKey::from(&private_key);
    let account_address = AuthenticationKey::ed25519(&public_key).account_address();

    // Build a local representation of an account.
    let account = LocalAccount::new(
        account_address,
        private_key,
        0,
    );

   let faucet_client = FaucetClient::new(FAUCET_URL.parse()?, RPC_URL.parse()?);
   faucet_client.fund(account_address, 100_000_000_000).await.unwrap();

    // Build an API client.
    let client = Client::new(RPC_URL.parse()?);
    let coin_client = CoinClient::new(&client);

    let balance = coin_client
                .get_account_balance(&account_address)
                .await.unwrap();

    // let balance = client.get_account_balance(account_address).await;
    println!("balance:{balance:?}", );

    //get account sequence numner
    let account_rpc = client.get_account(account_address).await.unwrap();
    let sequence_number = account_rpc.inner().sequence_number;
    println!("sequence_number: {sequence_number:?}", );

    //Call malicious script register for malicious_test::moon_coin::MoonCoin.
    let txn_builder = transaction_factory.script(Script::new(
        SCRIPT.to_vec(),
        // type args
        vec![],
        // args
        vec![
           // TransactionArgument::Address(account_address),
        ],
    )).sender(account_address).sequence_number(sequence_number);

   let raw_tx = txn_builder.build();

    // Build the transaction request and sign it.
    // Bug sign_with_transaction_builder don't use the sequence_number set in the build.
    // let signed_transaction = account.sign_with_transaction_builder(
    //     txn_builder
    // );
    let signed_transaction = account.sign_transaction(raw_tx);

    println!("signed_transaction:{signed_transaction:?}", );

   let base_max_gas_amount = signed_transaction.max_gas_amount();
   let base_gas_unit_price = signed_transaction.gas_unit_price();
   let base_expiration_timestamp_secs = signed_transaction.expiration_timestamp_secs();

    // Submit the transaction.
//    let _ = tokio::time::sleep(tokio::time::Duration::from_secs(1));
   let tx_receipt_data = client.submit_and_wait_bcs(&signed_transaction).await.unwrap();
   println!("tx_receipt_data: {tx_receipt_data:?}", );

    //Call mint EntryFunction to create some token
    let MemberId {
        module_id,
        member_id,
    } = str::parse("0xd82405d9faa256840ff6a8fe78d28d3f43581b1d34aa7f78476f4ce7e47a9e92::moon_coin::test").unwrap();
//    } = str::parse("0x1::managed_coin::mint").unwrap();

   let module_id1 = ModuleId::new(account_address, Identifier::new("moon_coin").unwrap());
   let member_id2 = Identifier::new("test").unwrap();
println!("module_id:{module_id:?}, member_id:{member_id:?}", );
    let payload =
        TransactionPayload::EntryFunction(EntryFunction::new(module_id, member_id, vec![], vec![])); //bcs::to_bytes(&account_address).unwrap(), bcs::to_bytes(&1000_000_000).unwrap()

        // TypeTag::Struct(Box::new(StructTag {
        //         address: account_address,
        //         module: Identifier::new("malicious_test").unwrap(),
        //         name: Identifier::new("moon_coin").unwrap(),
        //         type_params: vec![],
        //     }))

// async function mintCoin(minter: Account, receiverAddress: AccountAddress, amount: number): Promise<string> {
//   const transaction = await aptos.transaction.build.simple({
//     sender: minter.accountAddress,
//     data: {
//       function: "0x1::managed_coin::mint",
//       typeArguments: [`${minter.accountAddress}::moon_coin::MoonCoin`],
//       functionArguments: [receiverAddress, amount],
//     },
//   });

    let txn_builder = TransactionBuilder::new(payload, 30, ChainId::new(CHAIN_ID))
        .sender(account_address)
        .sequence_number(sequence_number+1)
        .max_gas_amount(base_max_gas_amount) // This is the minimum to execute this transaction
        .gas_unit_price(base_gas_unit_price)
        .expiration_timestamp_secs(base_expiration_timestamp_secs);
    let raw_tx = txn_builder.build();

    let MemberId {
        module_id,
        member_id,
    } = str::parse("0xd82405d9faa256840ff6a8fe78d28d3f43581b1d34aa7f78476f4ce7e47a9e92::moon_coin::test").unwrap();
    let entry_function =
        EntryFunction::new(module_id, member_id, vec![], vec![]); //bcs::to_bytes(&account_address).unwrap(), bcs::to_bytes(&1000_000_000).unwrap()
    let raw_tx = TransactionFactory::new(ChainId::new(CHAIN_ID)).entry_function(entry_function).sender(account_address).sequence_number(sequence_number+1).build();
    println!("raw_tx:{raw_tx:?}", );
    let signed_transaction = account.sign_transaction(raw_tx);
    println!("signed_transaction:{signed_transaction:?}", );
   let tx_receipt_data = client.submit_and_wait_bcs(&signed_transaction).await.unwrap();
    println!("RESULTTTTTTT tx_receipt_data: {tx_receipt_data:?}", );

//    let raw_tx = TransactionFactory::new(ChainId::new(CHAIN_ID)).mint(account_address, 1000).sender(account_address).sequence_number(sequence_number+1).build();

    let tytag = TypeTag::Struct(Box::new(StructTag {
                    address: account_address,
                    module: Identifier::new("moon_coin").unwrap(),
                    name: Identifier::new("MoonCoin").unwrap(),
                    type_params: vec![],
                }));

    let MemberId {
        module_id,
        member_id,
    } = str::parse("0x1::managed_coin::mint").unwrap();
    let entry_function =
        EntryFunction::new(module_id, member_id, vec![], vec![bcs::to_bytes(&account_address).unwrap(), bcs::to_bytes(&1000_000_000).unwrap()]); //bcs::to_bytes(&account_address).unwrap(), bcs::to_bytes(&1000_000_000).unwrap()
    
let entry_function = EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("aptos_coin").to_owned(),
        ),
        ident_str!("mint").to_owned(),
        vec![tytag],
        vec![
            bcs::to_bytes(&account_address).unwrap(),
            bcs::to_bytes(&1000_000_000).unwrap(),
        ],
    );

    let raw_tx = TransactionFactory::new(ChainId::new(CHAIN_ID)).entry_function(entry_function).sender(account_address).sequence_number(sequence_number+2).build();


    println!("raw_tx:{raw_tx:?}", );
    let signed_transaction = account.sign_transaction(raw_tx);
    println!("signed_transaction:{signed_transaction:?}", );

    println!("\n\n\n mint Tx");

    let tx_receipt_data = client.submit_and_wait_bcs(&signed_transaction).await.unwrap();
    println!("RESULTTTTTTT Mint tx_receipt_data: {tx_receipt_data:?}", );


    Ok(())
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

    async fn run_script(code: Vec<u8>,
    ty_args: Vec<TypeTag>,
    args: Vec<TransactionArgument>) -> Result<TransactionInfoV0> {
            //Call malicious script register for malicious_test::moon_coin::MoonCoin.
        let raw_tx = transaction_factory.script(Script::new(
            code,
            // type args
            ty_args, vec![],
            // args
            args, vec![],
        )).sender(account.address())
                .sequence_number(self.sequence_number).build();
        let signed_transaction = account.sign_transaction(raw_tx);

//    println!("signed_transaction:{signed_transaction:?}", );
        let res = self._submit_signed_tx(&signed_transaction)?;
        self.sequence_number +=1;
        Ok(res)
    }

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
        
        let res = self._submit_signed_tx(&signed_transaction)?;
        self.sequence_number +=1;
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