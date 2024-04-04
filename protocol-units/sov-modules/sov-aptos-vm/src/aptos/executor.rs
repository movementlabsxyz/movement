use crate::aptos::db::SovAptosDb;
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_db::AptosDB;
use aptos_sdk::move_types::vm_status::StatusCode;
use aptos_types::block_executor::partitioner::TxnIndex;
use aptos_types::state_store::StateView;
use aptos_types::transaction::{Transaction, TransactionOutput, Version, WriteSetPayload};
use aptos_types::vm_status::VMStatus;
use aptos_types::{
	chain_id::ChainId, transaction::signature_verified_transaction::SignatureVerifiedTransaction,
};
use aptos_vm::block_executor::AptosTransactionOutput;
use aptos_vm::data_cache::AsMoveResolver;
use aptos_vm::AptosVM;
use aptos_vm::VMExecutor;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{ExecutorView, ResourceGroupView};
use fail::fail_point;
use sov_modules_api::{StateMap, StateMapAccessor, StateValue, StateValueAccessor, WorkingSet};

pub(crate) struct AptosExecutor<'a, S> {
	vm: AptosVM,
	base_view: &'a S,
}
