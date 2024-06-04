use anyhow::Result;
use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;
use load_soak_testing::Scenario;
use dotenv::*;
use std::env;

use anyhow::{Error, Context, Result};
use aptos_sdk::{
    coin_client::CoinClient,
    rest_client::{
      Client, FaucetClient,
      aptos_api_types::{U64, ViewRequest, EntryFunctionId, VersionedEvent}
    },
    types::{LocalAccount, transaction::authenticator::AuthenticationKey},
    transaction_builder::TransactionBuilder,
    move_types::{
      ident_str,
      language_storage::{ModuleId, TypeTag},
    },
    crypto::{ed25519::{ Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature, PublicKey, PrivateKey}, ValidCryptoMaterialStringExt},
    transaction_builder::TransactionFactory,
    types::{
      account_address::AccountAddress,
      transaction::{
        EntryFunction, Script, SignedTransaction, TransactionArgument, TransactionPayload,
      }
    }
};

use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;
use tiny_keccak::{Hasher, Sha3};
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};


#[tokio::main]
async fn main() -> Result<()> {
    let mut path = env::current_dir().unwrap();
    path.push("../.env");
    from_path(&path).ok();

    let FULLNODE = env::var("FULLNODE").unwrap();
    let SWAP_DEPLOYER = env::var("SWAP_DEPLOYER").unwrap();
    let RESOURCE_ACCOUNT = env::var("RESOURCE_ACCOUNT_DEPLOYER").unwrap();
    let PRIVATE_KEY = env::var("PRIVATE_KEY").unwrap();

    // :!:>section_1a
    let aptos = Client::new(FULLNODE.clone());
    let faucet = FaucetClient::new(FULLNODE.clone(), FULLNODE.clone()); // <:!:section_1a

    // :!:>section_1b
    let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

    // Create two accounts locally, Alice and Bob.
    let mut deployer = LocalAccount::from_hex(&PRIVATE_KEY).unwrap();
    let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let bob = LocalAccount::generate(&mut rand::rngs::OsRng);
    let amount = 100000000;

  
    faucet.fundAccount(&alice.accountAddress(), amount).await?;
    faucet.fundAccount(&bob.accountAddress(), amount).await?;

    const fund = aptos.getAccountInfo({ account_address: alice.accountAddress() }).await?;
    println!("fund :{:?}", fund.inner());
    const modules = aptos.getModules({ account_address: deployer.accountAddress() }).await?;
    println!("modules :{:?}", modules.inner());
    const tokens = aptos.getAccountOwnedTokens({ account_address: alice.accountAddress() }).await?;
    println!("tokens :{:?}", tokens.inner());
}


#[derive(Serialize,Deserialize, Debug)]
struct DispatchEventData {
  dest_domain: u64,
  message: String,
  message_id: String,
  recipient: String,
  sender: String
}

#[derive(Serialize, Deserialize, Debug)]
struct ValidatorsAndThresholdMoveValue {
  validators: serde_json::Value,
  threshold: String,
}

fn main() {
	// Define the Test config. Use the default parameters.
	let config = ExecutionConfig::default();

	// Init the Test before execution
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}

	// Execute the test.
	let result = execute_test(config, &create_demo_scenario);
	tracing::info!("End Test with result {result:?}",);
}

// Scenario constructor function use by the Test runtime to create new scenarios.
fn create_demo_scenario(id: usize) -> Box<dyn Scenario> {
	Box::new(ScenarioDemo { id })
}

pub struct ScenarioDemo {
	id: usize,
}

impl ScenarioDemo {
	pub fn new(id: usize) -> Self {
		ScenarioDemo { id }
	}
}

// Scenario trait implementation.
#[async_trait::async_trait]
impl Scenario for ScenarioDemo {
	async fn run(self: Box<Self>) -> Result<usize> {
		// Trace in the log file and stdout.
		tracing::info!("Scenarios:{} start", self.id);
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		// Trace in the json formated execution log file.
		self.log_exec_info(&format!("Scenario:{} ended", self.id));
		Ok(self.id)
	}
}

