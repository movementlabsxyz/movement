//use anvil_core::eth::block;
use aptos_executor_test_helpers::{bootstrap_genesis, gen_block_id, gen_ledger_info_with_sigs};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_sdk::{
	transaction_builder::TransactionFactory,
	types::{AccountKey, LocalAccount},
};
use aptos_storage_interface::{state_view::DbStateViewAtVersion, DbReaderWriter, Order};
use aptos_types::transaction::{ModuleBundle, Transaction, TransactionPayload};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::{
	account_config::aptos_test_root_address,
	account_view::AccountView,
	block_metadata::BlockMetadata,
	chain_id::ChainId,
	event::EventKey,
	transaction::{
		Transaction::UserTransaction, TransactionListWithProof, TransactionWithProof,
		WriteSetPayload,
	},
	trusted_state::{TrustedState, TrustedStateChange},
	waypoint::Waypoint,
};
use sov_modules_api::{Context, CryptoSpec, Module, PrivateKey, PublicKey, Spec};
use sov_prover_storage_manager::new_orphan_storage;

use crate::call::CallMessage;
use crate::experimental::{AptosVmConfig, SovAptosVM};
use poem_openapi::__private::serde_json;
use rand_core::{RngCore, SeedableRng};
use sov_modules_api::utils::generate_address;
use sov_modules_api::Error;
use sov_state::storage::WorkingSet;
use sov_state::{DefaultStorageSpec, ProverStorage};
use sov_test_utils::MockZkVerifier;

type S = sov_modules_api::default_spec::DefaultSpec<MockZkVerifier>;

type DefaultPrivateKey = <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey;
const B: u64 = 1_000_000_000;

#[test]
fn serialize_deserialize_test() -> Result<(), Error> {
	// get validator_signer from aptosvm
	let signer = ValidatorSigner::from_int(0);
	// core resources account
	let mut core_resources_account: LocalAccount = LocalAccount::new(
		aptos_test_root_address(),
		AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
		0,
	);

	// transaction factory
	let tx_factory = TransactionFactory::new(ChainId::test());

	// accounts
	let account1 = LocalAccount::generate(&mut rand::rngs::OsRng);
	let account1_address = account1.address();
	let create1_tx = core_resources_account
		.sign_with_transaction_builder(tx_factory.create_user_account(account1.public_key()));
	let create1_txn = Transaction::UserTransaction(create1_tx);

	let serialized_tx = serde_json::to_vec::<Transaction>(&create1_txn).unwrap();
	let deserialized_tx = serde_json::from_slice::<Transaction>(&serialized_tx).unwrap();

	let call_message = CallMessage { serialized_txs: vec![serialized_tx] };
	let deserialized_tx_two = serde_json::from_slice::<Transaction>(
		&call_message.serialized_txs.get(0).expect("Empty serialized_txs"),
	)
	.unwrap();

	let block1_id = gen_block_id(1);
	let block1_meta_tx = Transaction::BlockMetadata(BlockMetadata::new(
		block1_id,
		1,
		0,
		signer.author(),
		vec![0],
		vec![],
		1,
	));

	let serialized_block_1tx = serde_json::to_vec::<Transaction>(&block1_meta_tx).unwrap();
	let deserialized_block1_tx =
		serde_json::from_slice::<Transaction>(&serialized_block_1tx).unwrap();

	let call_message_block1 = CallMessage { serialized_txs: vec![serialized_block_1tx] };
	let deserialized_block1_tx_too = serde_json::from_slice::<Transaction>(
		&call_message_block1.serialized_txs.get(0).expect("Empty serialized_txs"),
	)
	.unwrap();

	Ok(())
}

#[test]
fn aptosvm_small_test() -> Result<(), Error> {
	// seed
	let seed = [3u8; 32];
	let mut rng = ::rand::rngs::StdRng::from_seed(seed);

	// create a working set
	let tmpdir = tempfile::tempdir().unwrap();
	let storage = new_orphan_storage::<DefaultStorageSpec>(tmpdir.path()).unwrap();
	let mut working_set = WorkingSet::new(storage.clone());
	// sender context
	let priv_key = DefaultPrivateKey::generate();
	let sender = priv_key.pub_key();
	let admin = generate_address::<S>("admin");
	let sequencer = generate_address::<S>("sequencer");
	let admin_context = Context::<S>::new(admin, sequencer, 1);

	// initialize AptosVM
	let aptosvm = SovAptosVM::default();
	aptosvm.init_module(&AptosVmConfig { 
		data: vec![] ,
		path : tmpdir.path().to_path_buf()
	}, &mut working_set)?;

	// get validator_signer from aptosvm
	let signer = ValidatorSigner::from_int(0);
	// core resources account
	let mut core_resources_account: LocalAccount = LocalAccount::new(
		aptos_test_root_address(),
		AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
		0,
	);

	// transaction factory
	let tx_factory = TransactionFactory::new(ChainId::test());

	let mut account1 = LocalAccount::generate(&mut rng);
	let account1_address = account1.address();

	let create1_tx = Transaction::UserTransaction(
		core_resources_account
			.sign_with_transaction_builder(tx_factory.create_user_account(account1.public_key())),
	);

	let serialized_tx = serde_json::to_vec::<Transaction>(&create1_tx).unwrap();
	aptosvm
		.call(CallMessage { serialized_txs: vec![serialized_tx] }, &admin_context, &mut working_set)
		.unwrap();

	Ok(())
}

#[test]
fn aptosvm_test() -> Result<(), Error> {
	// seed
	let seed = [3u8; 32];
	let mut rng = ::rand::rngs::StdRng::from_seed(seed);

	// create a working set
	let tmpdir = tempfile::tempdir().unwrap();
	let storage = new_orphan_storage::<DefaultStorageSpec>(tmpdir.path()).unwrap();
	let mut working_set = WorkingSet::new(storage.clone());
	let admin = generate_address::<S>("admin");
	let sequencer = generate_address::<S>("sequencer");
	let admin_context = Context::<S>::new(admin, sequencer, 1);

	// sender context
	let priv_key = DefaultPrivateKey::generate();
	let sender = priv_key.pub_key();

	// initialize AptosVM
	let aptosvm = SovAptosVM::<S>::default();

	aptosvm.init_module(&AptosVmConfig {
		 data: vec![],
		 path : tmpdir.path().to_path_buf()
	}, &mut working_set)?;

	// get validator_signer from aptosvm
	let signer = ValidatorSigner::from_int(0);
	// core resources account
	let mut core_resources_account: LocalAccount = LocalAccount::new(
		aptos_test_root_address(),
		AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
		0,
	);

	// transaction factory
	let tx_factory = TransactionFactory::new(ChainId::test());

	// first block metadata
	let block1_id = gen_block_id(1);
	let block1_meta_tx = Transaction::BlockMetadata(BlockMetadata::new(
		block1_id,
		1,
		0,
		signer.author(),
		vec![0],
		vec![],
		1,
	));

	// accounts
	let mut account1 = LocalAccount::generate(&mut rng);
	let mut account2 = LocalAccount::generate(&mut rng);
	let mut account3 = LocalAccount::generate(&mut rng);
	let account1_address = account1.address();
	let account2_address = account2.address();
	let account3_address = account3.address();

	// create accounts
	let create1_tx = core_resources_account
		.sign_with_transaction_builder(tx_factory.create_user_account(account1.public_key()));
	let create2_tx = core_resources_account
		.sign_with_transaction_builder(tx_factory.create_user_account(account2.public_key()));

	// Create account1 with 2T coins.
	let coins1_tx = core_resources_account
		.sign_with_transaction_builder(tx_factory.mint(account1.address(), 2_000 * B));
	// Create account2 with 1.2T coins.
	let coins2_tx = core_resources_account
		.sign_with_transaction_builder(tx_factory.mint(account2.address(), 1_200 * B));

	// Transfer 20B coins from account1 to account2.
	// balance: <1.98T, 1.22T, 1T
	let transfer_1_2_tx =
		account1.sign_with_transaction_builder(tx_factory.transfer(account2.address(), 20 * B));

	// use transaction factory to create module bundle
	/* let path = PathBuf::from("modules/sov-aptosvm/src/tests/contracts/SimpleStorage.mv");
	let package = BuiltPackage::build(path.to_owned(), BuildOptions::default())
			.expect("building package must succeed");
	let code = package.extract_code();

	let create_module_tx = account1.sign_with_transaction_builder(
		tx_factory.payload(TransactionPayload::ModuleBundle(ModuleBundle::new(
			code,
		)))
	);*/

	// use transaction factory to create entrypoint call
	let block_vec: Vec<Transaction> = vec![
		UserTransaction(create1_tx),
		UserTransaction(create2_tx),
		UserTransaction(coins1_tx),
		UserTransaction(coins2_tx),
		UserTransaction(transfer_1_2_tx.clone()),
		// Transaction::UserTransaction(create_module_tx),
	];

	let mut serialized_txs = vec![];

	// for transaction in the above
	for tx in block_vec {
		let serialized_tx = serde_json::to_vec::<Transaction>(&tx).unwrap();
		// call the transaction
		serialized_txs.push(serialized_tx);
	}

	aptosvm
		.call(CallMessage { serialized_txs }, &admin_context, &mut working_set)
		.unwrap();

	let block_vec_two: Vec<Transaction> = vec![
		UserTransaction(transfer_1_2_tx.clone()),
		UserTransaction(transfer_1_2_tx.clone()),
		UserTransaction(transfer_1_2_tx.clone()),
		// Transaction::UserTransaction(create_module_tx),
	];

	let mut serialized_txs_two = vec![];

	// for transaction in the above
	for tx in block_vec_two {
		let serialized_tx = serde_json::to_vec::<Transaction>(&tx).unwrap();
		// call the transaction
		serialized_txs_two.push(serialized_tx);
	}

	aptosvm
		.call(CallMessage { serialized_txs: serialized_txs_two }, &admin_context, &mut working_set)
		.unwrap();

	// check caller address

	// check contract address

	// check contract storage

	// let db_account = evm.accounts.get(&contract_addr, working_set).unwrap();
	// let storage_key = &[0; 32];
	// let storage_value = db_account.storage.get(storage_key, working_set).unwrap();

	// assert_eq!(set_arg.to_le_bytes(), storage_value[0..4])

	Ok(())
}
