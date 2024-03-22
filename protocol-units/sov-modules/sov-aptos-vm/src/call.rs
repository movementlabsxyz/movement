use anyhow::Result;
use reth_primitives::{Log as RethLog, TransactionSignedEcRecovered};
use revm::primitives::{Address, CfgEnv, CfgEnvWithHandlerCfg, EVMError, Log, SpecId};
use sov_modules_api::{CallResponse, Context, DaSpec, WorkingSet, StateVecAccessor, StateValueAccessor};

use crate::evm::db::AptosDb;
use crate::evm::executor::{self};
use crate::evm::primitive_types::{BlockEnv, Receipt, TransactionSignedAndRecovered};
use crate::evm::{AptosChainConfig, RlpEvmTransaction};
use crate::experimental::{AptosVM, PendingTransaction};

/// EVM call message.
#[derive(
borsh::BorshDeserialize,
borsh::BorshSerialize,
serde::Serialize,
serde::Deserialize,
Debug,
PartialEq,
Clone,
)]
pub struct CallMessage {
    /// RLP encoded transaction.
    pub tx: RlpEvmTransaction,
}

impl<S: sov_modules_api::Spec, Da: DaSpec> AptosVM<S, Da> {
    pub(crate) fn execute_call(
        &self,
        tx: RlpEvmTransaction,
        _context: &Context<S>,
        working_set: &mut WorkingSet<S>,
    ) -> Result<CallResponse> {
        let evm_tx_recovered: TransactionSignedEcRecovered = tx.try_into()?;
        let block_env = self
            .block_env
            .get(working_set)
            .expect("Pending block must be set");

        let cfg = self.cfg.get(working_set).expect("Evm config must be set");
        let cfg_env = get_cfg_env_with_handler(&block_env, cfg, None);

        let aptos_db: AptosDb<'_, S> = self.get_db(working_set);
        let result = executor::execute_tx(aptos_db, &block_env, &evm_tx_recovered, cfg_env);
        let previous_transaction = self.pending_transactions.last(working_set);
        let previous_transaction_cumulative_gas_used = previous_transaction
            .as_ref()
            .map_or(0u64, |tx| tx.receipt.receipt.cumulative_gas_used);
        let log_index_start = previous_transaction.as_ref().map_or(0u64, |tx| {
            tx.receipt.log_index_start + tx.receipt.receipt.logs.len() as u64
        });

        let receipt = match result {
            Ok(result) => {
                let logs: Vec<_> = result.logs().into_iter().map(into_reth_log).collect();
                let gas_used = result.gas_used();
                tracing::debug!(
                    hash = hex::encode(evm_tx_recovered.hash()),
                    gas_used,
                    "EVM transaction has been successfully executed"
                );
                Receipt {
                    receipt: reth_primitives::Receipt {
                        tx_type: evm_tx_recovered.tx_type(),
                        success: result.is_success(),
                        cumulative_gas_used: previous_transaction_cumulative_gas_used + gas_used,
                        logs,
                    },
                    gas_used,
                    log_index_start,
                    error: None,
                }
            }
            // Adopted from https://github.com/paradigmxyz/reth/blob/main/crates/payload/basic/src/lib.rs#L884
            Err(err) => {
                tracing::debug!(
                    tx_hash = hex::encode(evm_tx_recovered.hash()),
                    error = ?err,
                    "EVM transaction has been reverted"
                );
                return match err {
                    EVMError::Transaction(_) => {
                        // This is a transactional error, so we can skip it without doing anything.
                        Ok(CallResponse::default())
                    }
                    err => {
                        // This is a fatal error, so we need to return it.
                        Err(err.into())
                    }
                };
            }
        };

        // let pending_transaction = PendingTransaction {
        //     transaction: TransactionSignedAndRecovered {
        //         signer: evm_tx_recovered.signer(),
        //         signed_transaction: evm_tx_recovered.into(),
        //         block_number: block_env.number,
        //     },
        //     receipt,
        // };
        //
        // self.pending_transactions
        //     .push(&pending_transaction, working_set);
        //
        // Ok(CallResponse::default())
        todo!()
    }
}

/// builds CfgEnvWithHandlerCfg
/// Returns correct config depending on spec for given block number
// Copies context-dependent values from template_cfg or default if not provided
pub(crate) fn get_cfg_env_with_handler(
    block_env: &BlockEnv,
    cfg: AptosChainConfig,
    template_cfg: Option<CfgEnv>,
) -> CfgEnvWithHandlerCfg {
    todo!()
}

/// Get spec id for a given block number
/// Returns the first spec id defined for block >= block_number
pub(crate) fn get_spec_id(spec: Vec<(u64, SpecId)>, block_number: u64) -> u64 {
    // not sure we need this for sov-aptos, the values can be hardcoded
    todo!()
}

/// Copied from <https://github.com/paradigmxyz/reth/blob/e83d3aa704f87825ca8cab6f593ab4d4adbf6792/crates/revm/revm-primitives/src/compat.rs#L17-L23>.
/// All rights reserved.
///
/// By copying the code, we can avoid depending on the whole crate.
pub fn into_reth_log(log: Log) -> RethLog {
    RethLog {
        address: Address(log.address.0),
        topics: log.topics().to_vec(),
        data: log.data.data,
    }
}
