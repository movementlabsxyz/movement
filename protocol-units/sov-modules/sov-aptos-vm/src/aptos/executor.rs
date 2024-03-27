use sov_modules_api::{StateMap, StateMapAccessor, StateValue, StateValueAccessor, WorkingSet};
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
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_vm::block_executor::AptosTransactionOutput;
use aptos_vm::data_cache::AsMoveResolver;
use aptos_vm::{, VMExecutor}
use aptos_vm::AptosVM;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::output::VMOutput;
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_vm_types::resolver::{ExecutorView, ResourceGroupView};
use fail::fail_point;
use crate::aptos::db::{DbStateView, SovAptosDb};

pub(crate) struct AptosExecutor<'a, S> {
	vm: AptosVM,
	base_view: &'a S,
}

impl<'a, S: 'a + StateView + Sync> ExecutorTask for AptosExecutor<'a, S> {
	type Txn = SignatureVerifiedTransaction;
	type Output = AptosTransactionOutput;
	type Error = VMStatus;
	type Argument = &'a S;

	fn init(argument: &'a S) -> Self {
		// AptosVM has to be initialized using configs from storage
		let vm = AptosVM::new(&argument.as_move_resolver(), Some(true));
		Self { vm, base_view: argument }
	}

	// This function is called by the BlockExecutor for each transaction is intends
	// to execute (via the ExecutorTask trait). It can be as a part of sequential
	// execution, or speculatively as a part of a parallel execution.
	fn execute_transaction(
		&self,
		executor_with_group_view: &(impl ExecutorView + ResourceGroupView),
		txn: &SignatureVerifiedTransaction,
		txn_idx: u32,
	) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
		fail_point!("aptos_vm::vm_wrapper::execute_transaction", |_| {
			ExecutionStatus::DelayedFieldsCodeInvariantError("fail points error".into())
		});

		let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx as usize);
		let resolver = self.vm.as_move_resolver_with_group_view(executor_with_group_view);

		match self.vm.execute_single_transaction(txn, &resolver, &log_context) {
			Ok((vm_status, vm_output)) => {
				if vm_status.status_code() == StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR {
					ExecutionStatus::SpeculativeExecutionAbortError(
						vm_status.message().cloned().unwrap_or_default(),
					)
				} else if vm_status.status_code()
					== StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR
				{
					ExecutionStatus::DelayedFieldsCodeInvariantError(
						vm_status.message().cloned().unwrap_or_default(),
					)
				} else {
					assert!(
						Self::is_transaction_dynamic_change_set_capable(txn),
						"DirectWriteSet should always create SkipRest transaction, validate_waypoint_change_set provides this guarantee"
					);
					ExecutionStatus::Success(AptosTransactionOutput::new(vm_output))
				}
			},
			// execute_single_transaction only returns an error when transactions that should never fail
			// (BlockMetadataTransaction and GenesisTransaction) return an error themselves.
			Err(err) => {
				if err.status_code() == StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR {
					ExecutionStatus::SpeculativeExecutionAbortError(
						err.message().cloned().unwrap_or_default(),
					)
				} else if err.status_code()
					== StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR
				{
					ExecutionStatus::DelayedFieldsCodeInvariantError(
						err.message().cloned().unwrap_or_default(),
					)
				} else {
					ExecutionStatus::Abort(err)
				}
			},
		}
	}

	fn is_transaction_dynamic_change_set_capable(txn: &Self::Txn) -> bool {
		if txn.is_valid() {
			if let Transaction::GenesisTransaction(WriteSetPayload::Direct(_)) = txn.expect_valid()
			{
				// WriteSetPayload::Direct cannot be handled in mode where delayed_field_optimization or
				// resource_groups_split_in_change_set is enabled.
				return false;
			}
		}
		true
	}
}

pub fn execute_block_no_limit<S>(
	state: &SovAptosDb<S>,
	transactions: &[SignatureVerifiedTransaction],
) -> Result<Vec<TransactionOutput>, VMStatus>
	where S: sov_modules_api::Spec
{
	AptosVM::execute_block_no_limit(transactions, state)
}
