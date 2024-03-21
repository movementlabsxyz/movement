use std::array::TryFromSliceError;

use aptos_consensus_types::common::Payload;
use aptos_sdk::types::account_address::AccountAddress;
use jsonrpsee::core::RpcResult;
use reth_primitives::{TransactionKind, TransactionSignedEcRecovered, U128, U64};
use reth_rpc_types_compat::block::from_primitive_with_hash;
use reth_rpc_types_compat::transaction::from_recovered_with_block_context;
use revm::primitives::{
    Address, EVMError, ExecutionResult, HaltReason, InvalidTransaction, TransactTo, B256,
    KECCAK_EMPTY, U256,
};
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::{DaSpec, StateMapAccessor, StateValueAccessor, WorkingSet};
use tracing::debug;

use crate::evm::primitive_types::BlockTransactions;
use aptos_consensus_types::block::Block;
use aptos_crypto::bls12381::Signature;

use crate::call::get_cfg_env_with_handler;
use crate::evm::db::AptosDb;
use crate::evm::error::rpc::{RevertError, RpcInvalidTransactionError};
use crate::evm::executor;
use crate::evm::primitive_types::{
    BlockEnv, Receipt, SealedBlock, SovAptosBlock, TransactionSignedAndRecovered,
};
use crate::experimental::AptosVM;
use crate::helpers::prepare_call_env;

#[rpc_gen(client, server)]
impl<S: sov_modules_api::Spec, Da: DaSpec> AptosVM<S, Da> {
    /// Handler for `net_version`
    #[rpc_method(name = "get_ledger_info")]
    pub fn net_version(&self, working_set: &mut WorkingSet<S>) -> RpcResult<String> {
        debug!("Aptos VM module JSON-RPC request to `get_ledger_info`");

        // Network ID is the same as chain ID for most networks
        // Not sure if this is the same for Aptos, unit test this.
        let chain_id = self
            .cfg
            .get(working_set)
            .expect("AptosVM config must be set at genesis")
            .chain_id;

        Ok(chain_id.to_string())
    }

    /// Handler for: `healthy`
    #[rpc_method(name = "healthy")]
    pub fn chain_id(&self, working_set: &mut WorkingSet<S>) -> RpcResult<Option<U64>> {
        let chain_id = self
            .cfg
            .get(working_set)
            .expect("AptosVM config must be set at genesis")
            .chain_id;
        debug!(
            chain_id = chain_id,
            "AptosVM module JSON-RPC request to `healthy`"
        );
        Ok(Some(U64::from(chain_id)))
    }

    /// Handler for `get_block_by_signature`
    #[rpc_method(name = "get_block_by_signature")]
    pub fn get_block_by_signature(
        &self,
        signature: Signature,
        details: Option<bool>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
        debug!(
            ?signature,
            "AptosVM module JSON-RPC request to `get_block_by_signature`"
        );

        let block_number_hex = self
            .block_hashes
            .get(&signature, &mut working_set.accessory_state())
            .map(|number| hex::encode(number.to_be_bytes()))
            .expect("Block number for known block hash must be set");

        self.get_block_by_height(Some(block_number_hex), details, working_set)
    }

    /// Handler for: `get_block_by_height`
    #[rpc_method(name = "get_block_by_height")]
    pub fn get_block_by_height(
        &self,
        block_number: Option<String>,
        details: Option<bool>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
        debug!(
            block_number,
            "AptosVM module JSON-RPC request to `get_block_by_height`"
        );

        let block: Block = self.get_sealed_block_by_number(block_number, working_set);

        // Build rpc header response
        let header = from_primitive_with_hash(block.header.clone());

        let payload = block.payload().expect("No payload in block");
        let transactions = match payload {
            Payload::DirectMempool(txs) => txs,
            _ => panic!("Only DirectMempool payload is supported"), // add proper error
        };

        let block = SovAptosBlock {
            block,
            transactions: BlockTransactions::Full(transactions),
        };

        Ok(Some(block.into()))
    }

    /// Handler for: `get_balance`
    #[rpc_method(name = "get_balance")]
    pub fn get_balance(
        &self,
        address: AccountAddress,
        _block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<U256> {
        // TODO: Implement block_number once we have archival state #951
        // https://github.com/Sovereign-Labs/sovereign-sdk/issues/951

        let balance = self
            .accounts
            .get(&address.to_standard_string(), working_set)
            .map(|account| account.info.balance)
            .unwrap_or_default();

        debug!(
            %address,
            %balance,
            "AptosVM module JSON-RPC request to `get_balance`"
        );

        Ok(balance)
    }

    /// Handler for: `eth_getStorageAt`
    #[rpc_method(name = "eth_getStorageAt")]
    pub fn get_storage_at(
        &self,
        address: Address,
        index: U256,
        _block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<U256> {
        debug!("EVM module JSON-RPC request to `eth_getStorageAt`");

        // TODO: Implement block_number once we have archival state #951
        // https://github.com/Sovereign-Labs/sovereign-sdk/issues/951

        let storage_slot = self
            .accounts
            .get(&address, working_set)
            .and_then(|account| account.storage.get(&index, working_set))
            .unwrap_or_default();

        Ok(storage_slot)
    }

    /// Handler for: `eth_getTransactionCount`
    #[rpc_method(name = "eth_getTransactionCount")]
    pub fn get_transaction_count(
        &self,
        address: Address,
        _block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<U64> {
        // TODO: Implement block_number once we have archival state #882
        // https://github.com/Sovereign-Labs/sovereign-sdk/issues/882

        let nonce = self
            .accounts
            .get(&address, working_set)
            .map(|account| account.info.nonce)
            .unwrap_or_default();

        debug!(%address, nonce, "EVM module JSON-RPC request to `eth_getTransactionCount`");

        Ok(U64::from(nonce))
    }

    /// Handler for: `eth_getCode`
    #[rpc_method(name = "eth_getCode")]
    pub fn get_code(
        &self,
        address: Address,
        _block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<reth_primitives::Bytes> {
        debug!("EVM module JSON-RPC request to `eth_getCode`");

        // TODO: Implement block_number once we have archival state #951
        // https://github.com/Sovereign-Labs/sovereign-sdk/issues/951

        let code = self
            .accounts
            .get(&address, working_set)
            .and_then(|account| self.code.get(&account.info.code_hash, working_set))
            .unwrap_or_default();

        Ok(code)
    }

    /// Handler for: `eth_feeHistory`
    // TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
    #[rpc_method(name = "eth_feeHistory")]
    pub fn fee_history(&self) -> RpcResult<reth_rpc_types::FeeHistory> {
        debug!("EVM module JSON-RPC request to `eth_feeHistory`");

        Ok(reth_rpc_types::FeeHistory {
            base_fee_per_gas: Default::default(),
            gas_used_ratio: Default::default(),
            oldest_block: Default::default(),
            reward: Default::default(),
            blob_gas_used_ratio: Default::default(),
            // EIP-4844 related
            base_fee_per_blob_gas: Default::default(),
        })
    }

    /// Handler for: `eth_getTransactionByHash`
    // TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
    #[rpc_method(name = "get_transaction_by_hash")]
    pub fn get_transaction_by_hash(
        &self,
        hash: B256,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<Option<reth_rpc_types::Transaction>> {
        let mut accessory_state = working_set.accessory_state();

        let tx_number = self.transaction_hashes.get(&hash, &mut accessory_state);

        let transaction = tx_number.map(|number| {
            let tx = self
                .transactions
                .get(number as usize, &mut accessory_state)
                .unwrap_or_else(|| panic!("Transaction with known hash {} and number {} must be set in all {} transaction",
                                          hash,
                                          number,
                                          self.transactions.len(&mut accessory_state)));

            let block = self
                .blocks
                .get(tx.block_number as usize, &mut accessory_state)
                .unwrap_or_else(|| panic!("Block with number {} for known transaction {} must be set",
                                          tx.block_number,
                                          tx.signed_transaction.hash));

            from_recovered_with_block_context(
                tx.into(),
                block.header.hash(),
                block.header.number,
                block.header.base_fee_per_gas,
                U256::from(tx_number.unwrap() - block.transactions.start),
            )
        });

        debug!(
            %hash,
            ?transaction,
            "EVM module JSON-RPC request to `eth_getTransactionByHash`"
        );

        Ok(transaction)
    }

    /// Handler for: `eth_getTransactionReceipt`
    // TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
    #[rpc_method(name = "eth_getTransactionReceipt")]
    pub fn get_transaction_receipt(
        &self,
        hash: B256,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<Option<reth_rpc_types::TransactionReceipt>> {
        debug!(
            %hash,
            "EVM module JSON-RPC request to `eth_getTransactionReceipt`"
        );

        let mut accessory_state = working_set.accessory_state();

        let tx_number = self.transaction_hashes.get(&hash, &mut accessory_state);

        let receipt = tx_number.map(|number| {
            let tx = self
                .transactions
                .get(number as usize, &mut accessory_state)
                .expect("Transaction with known hash must be set");
            let block = self
                .blocks
                .get(tx.block_number as usize, &mut accessory_state)
                .expect("Block number for known transaction must be set");

            let receipt = self
                .receipts
                .get(tx_number.unwrap() as usize, &mut accessory_state)
                .expect("Receipt for known transaction must be set");

            build_rpc_receipt(block, tx, tx_number.unwrap(), receipt)
        });

        Ok(receipt)
    }

    /// Handler for: `eth_call`
    //https://github.com/paradigmxyz/reth/blob/f577e147807a783438a3f16aad968b4396274483/crates/rpc/rpc/src/eth/api/transactions.rs#L502
    //https://github.com/paradigmxyz/reth/blob/main/crates/rpc/rpc-types/src/eth/call.rs#L7
    #[rpc_method(name = "eth_call")]
    pub fn get_call(
        &self,
        request: reth_rpc_types::TransactionRequest,
        block_number: Option<String>,
        _state_overrides: Option<reth_rpc_types::state::StateOverride>,
        _block_overrides: Option<Box<reth_rpc_types::BlockOverrides>>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<reth_primitives::Bytes> {
        debug!("EVM module JSON-RPC request to `eth_call`");

        let block_env = match block_number {
            Some(ref block_number) if block_number == "pending" => {
                self.block_env.get(working_set).unwrap_or_default().clone()
            }
            _ => {
                let block = self.get_sealed_block_by_number(block_number, working_set);
                BlockEnv::from(&block)
            }
        };

        let tx_env = prepare_call_env(&block_env, request.clone()).unwrap();

        let cfg = self.cfg.get(working_set).unwrap_or_default();
        let cfg_env = get_cfg_env_with_handler(&block_env, cfg, Some(get_cfg_env_template()));

        let aptos_db: AptosDb<'_, S> = self.get_db(working_set);

        let result = match executor::inspect(evm_db, &block_env, tx_env, cfg_env) {
            Ok(result) => result.result,
            Err(err) => return Err(EthApiError::from(err).into()),
        };

        Ok(ensure_success(result)?)
    }

    /// Handler for: `eth_blockNumber`
    #[rpc_method(name = "eth_blockNumber")]
    pub fn block_number(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U256> {
        debug!("EVM module JSON-RPC request to `eth_blockNumber`");

        Ok(U256::from(
            self.blocks
                .len(&mut working_set.accessory_state())
                .saturating_sub(1),
        ))
    }

    /// Handler for: `eth_estimateGas`
    // https://github.com/paradigmxyz/reth/blob/main/crates/rpc/rpc/src/eth/api/call.rs#L172
    #[rpc_method(name = "eth_estimateGas")]
    pub fn eth_estimate_gas(
        &self,
        request: reth_rpc_types::TransactionRequest,
        block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> RpcResult<U64> {
        debug!("EVM module JSON-RPC request to `eth_estimateGas`");
        let mut block_env = match block_number {
            Some(ref block_number) if block_number == "pending" => {
                self.block_env.get(working_set).unwrap_or_default().clone()
            }
            _ => {
                let block = self.get_sealed_block_by_number(block_number, working_set);
                BlockEnv::from(&block)
            }
        };

        let tx_env = prepare_call_env(&block_env, request.clone()).unwrap();

        let cfg = self.cfg.get(working_set).unwrap_or_default();
        let cfg_env_with_handler =
            get_cfg_env_with_handler(&block_env, cfg, Some(get_cfg_env_template()));

        let request_gas = request.gas;
        let request_gas_price = request.gas_price;
        let env_gas_limit = block_env.gas_limit;

        // get the highest possible gas limit, either the request's set value or the currently
        // configured gas limit
        let mut highest_gas_limit = request.gas.unwrap_or(U256::from(env_gas_limit));

        let account = self
            .accounts
            .get(&tx_env.caller, working_set)
            .map(|account| account.info)
            .unwrap_or_default();

        // if the request is a simple transfer, can we optimize?
        if tx_env.data.is_empty() {
            if let TransactTo::Call(to) = tx_env.transact_to {
                let to_account = self
                    .accounts
                    .get(&to, working_set)
                    .map(|account| account.info)
                    .unwrap_or_default();
                if KECCAK_EMPTY == to_account.code_hash {
                    // simple transfer, check if caller has sufficient funds
                    let available_funds = account.balance;

                    if tx_env.value > available_funds {
                        return Err(RpcInvalidTransactionError::InsufficientFundsForTransfer.into());
                    }
                    return Ok(U64::from(MIN_TRANSACTION_GAS));
                }
            }
        }

        // check funds of the sender
        if tx_env.gas_price > U256::ZERO {
            // allowance is (balance - tx.value) / tx.gas_price
            let allowance = (account.balance - tx_env.value) / tx_env.gas_price;

            if highest_gas_limit > allowance {
                // cap the highest gas limit by max gas caller can afford with a given gas price
                highest_gas_limit = allowance;
            }
        }

        // if the provided gas limit is less than computed cap, use that
        let gas_limit = std::cmp::min(U256::from(tx_env.gas_limit), highest_gas_limit);
        block_env.gas_limit = convert_u256_to_u64(gas_limit).unwrap();

        let evm_db = self.get_db(working_set);

        // execute the call without writing to db
        let result = executor::inspect(
            evm_db,
            &block_env,
            tx_env.clone(),
            cfg_env_with_handler.clone(),
        );

        // Exceptional case: init used too much gas, we need to increase the gas limit and try
        // again
        if let Err(EVMError::Transaction(InvalidTransaction::CallerGasLimitMoreThanBlock)) = result
        {
            // if price or limit was included in the request, then we can execute the request
            // again with the block's gas limit to check if revert is gas related or not
            if request_gas.is_some() || request_gas_price.is_some() {
                let evm_db = self.get_db(working_set);
                return Err(
                    map_out_of_gas_err(block_env, tx_env, cfg_env_with_handler, evm_db).into(),
                );
            }
        }

        let result = match result {
            Ok(result) => match result.result {
                ExecutionResult::Success { .. } => result.result,
                ExecutionResult::Halt { reason, gas_used } => {
                    return Err(RpcInvalidTransactionError::halt(reason, gas_used).into());
                }
                ExecutionResult::Revert { output, .. } => {
                    // if price or limit was included in the request,
                    // then we can execute the request
                    // again with the block's gas limit to check if revert is gas related or not
                    return if request_gas.is_some() || request_gas_price.is_some() {
                        let evm_db = self.get_db(working_set);
                        Err(
                            map_out_of_gas_err(block_env, tx_env, cfg_env_with_handler, evm_db)
                                .into(),
                        )
                    } else {
                        // the transaction did revert
                        Err(
                            RpcInvalidTransactionError::Revert(RevertError::new(output.into()))
                                .into(),
                        )
                    };
                }
            },
            Err(err) => return Err(EthApiError::from(err).into()),
        };

        // at this point, we know the call succeeded but want to find the _best_ (lowest) gas the
        // transaction succeeds with.
        // we find this by doing a binary search over the
        // possible range NOTE: this is the gas the transaction used, which is less than the
        // transaction requires succeeding
        let gas_used = result.gas_used();
        // the lowest value is capped by the gas it takes for a transfer
        let mut lowest_gas_limit = if tx_env.transact_to.is_create() {
            MIN_CREATE_GAS
        } else {
            MIN_TRANSACTION_GAS
        };
        let mut highest_gas_limit: u64 = highest_gas_limit.try_into().unwrap_or(u64::MAX);
        // pick a point that's close to the estimated gas
        let mut mid_gas_limit = std::cmp::min(
            gas_used * 3,
            ((highest_gas_limit as u128 + lowest_gas_limit as u128) / 2) as u64,
        );
        // binary search
        while (highest_gas_limit - lowest_gas_limit) > 1 {
            let mut tx_env = tx_env.clone();
            tx_env.gas_limit = mid_gas_limit;

            let evm_db = self.get_db(working_set);
            let result = executor::inspect(
                evm_db,
                &block_env,
                tx_env.clone(),
                cfg_env_with_handler.clone(),
            );

            // Exceptional case: init used too much gas, we need to increase the gas limit and try
            // again
            if let Err(EVMError::Transaction(InvalidTransaction::CallerGasLimitMoreThanBlock)) =
                result
            {
                // increase the lowest gas limit
                lowest_gas_limit = mid_gas_limit;

                // new midpoint
                mid_gas_limit = ((highest_gas_limit as u128 + lowest_gas_limit as u128) / 2) as u64;
                continue;
            }

            match result {
                Ok(result) => match result.result {
                    ExecutionResult::Success { .. } => {
                        // cap the highest gas limit with succeeding gas limit
                        highest_gas_limit = mid_gas_limit;
                    }
                    ExecutionResult::Revert { .. } => {
                        // increase the lowest gas limit
                        lowest_gas_limit = mid_gas_limit;
                    }
                    ExecutionResult::Halt { reason, .. } => {
                        match reason {
                            HaltReason::OutOfGas(_) => {
                                // increase the lowest gas limit
                                lowest_gas_limit = mid_gas_limit;
                            }
                            err => {
                                // these should be unreachable because we know the transaction succeeds,
                                // but we consider these cases an error
                                return Err(RpcInvalidTransactionError::EvmHalt(err).into());
                            }
                        }
                    }
                },
                Err(err) => return Err(EthApiError::from(err).into()),
            };

            // new midpoint
            mid_gas_limit = ((highest_gas_limit as u128 + lowest_gas_limit as u128) / 2) as u64;
        }

        Ok(U64::from(highest_gas_limit))
    }

    fn get_sealed_block_by_number(
        &self,
        block_number: Option<String>,
        working_set: &mut WorkingSet<S>,
    ) -> Block {
        // safe, finalized, and pending are not supported
        match block_number {
            Some(ref block_number) if block_number == "earliest" => self
                .blocks
                .get(0, &mut working_set.accessory_state())
                .expect("Genesis block must be set"),
            Some(ref block_number) if block_number == "latest" => self
                .blocks
                .last(&mut working_set.accessory_state())
                .expect("Head block must be set"),
            Some(ref block_number) => {
                // hex representation may have 0x prefix
                let block_number = usize::from_str_radix(block_number.trim_start_matches("0x"), 16)
                    .expect("Block number must be a valid hex number, with or without 0x prefix");
                self.blocks
                    .get(block_number, &mut working_set.accessory_state())
                    .expect("Block must be set")
            }
            None => self
                .blocks
                .last(&mut working_set.accessory_state())
                .expect("Head block must be set"),
        }
    }
}

fn get_cfg_env_template() -> revm::primitives::CfgEnv {
    let mut cfg_env = revm::primitives::CfgEnv::default();
    // Reth sets this to true and uses only timeout, but other clients use this as a part of DOS attacks protection, with 100mln gas limit
    // https://github.com/paradigmxyz/reth/blob/62f39a5a151c5f4ddc9bf0851725923989df0412/crates/rpc/rpc/src/eth/revm_utils.rs#L215
    cfg_env.disable_block_gas_limit = false;
    cfg_env.disable_eip3607 = true;
    cfg_env.disable_base_fee = true;
    cfg_env.chain_id = 0;
    cfg_env.perf_analyse_created_bytecodes = revm::primitives::AnalysisKind::Analyse;
    cfg_env.limit_contract_code_size = None;
    cfg_env
}

// modified from: https://github.com/paradigmxyz/reth/blob/cc576bc8690a3e16e6e5bf1cbbbfdd029e85e3d4/crates/rpc/rpc/src/eth/api/transactions.rs#L849
pub(crate) fn build_rpc_receipt(
    block: SealedBlock,
    tx: TransactionSignedAndRecovered,
    tx_number: u64,
    receipt: Receipt,
) -> reth_rpc_types::TransactionReceipt {
    let transaction: TransactionSignedEcRecovered = tx.into();
    let transaction_kind = transaction.kind();

    let transaction_hash = Some(transaction.hash);
    let transaction_index = tx_number - block.transactions.start;
    let block_hash = Some(block.header.hash());
    let block_number = Some(U256::from(block.header.number));

    reth_rpc_types::TransactionReceipt {
        transaction_hash,
        transaction_index: U64::from(transaction_index),
        block_hash,
        block_number,
        from: transaction.signer(),
        to: match transaction_kind {
            TransactionKind::Create => None,
            TransactionKind::Call(addr) => Some(*addr),
        },
        cumulative_gas_used: U256::from(receipt.receipt.cumulative_gas_used),
        gas_used: Some(U256::from(receipt.gas_used)),
        // EIP-4844 related
        // https://github.com/Sovereign-Labs/sovereign-sdk/issues/912
        blob_gas_used: None,
        blob_gas_price: None,
        contract_address: match transaction_kind {
            TransactionKind::Create => Some(transaction.signer().create(transaction.nonce())),
            TransactionKind::Call(_) => None,
        },
        effective_gas_price: U128::from(
            transaction.effective_gas_price(block.header.base_fee_per_gas),
        ),
        transaction_type: transaction.tx_type().into(),
        logs_bloom: receipt.receipt.bloom_slow(),
        status_code: if receipt.receipt.success {
            Some(U64::from(1))
        } else {
            Some(U64::from(0))
        },
        state_root: None, // Pre https://eips.ethereum.org/EIPS/eip-658 (pre-byzantium) and won't be used
        logs: receipt
            .receipt
            .logs
            .into_iter()
            .enumerate()
            .map(|(idx, log)| reth_rpc_types::Log {
                address: log.address,
                topics: log.topics,
                data: log.data,
                block_hash,
                block_number,
                transaction_hash,
                transaction_index: Some(U256::from(transaction_index)),
                log_index: Some(U256::from(receipt.log_index_start + idx as u64)),
                removed: false,
            })
            .collect(),
        // TODO: can we put actual value
        other: Default::default(),
    }
}

fn map_out_of_gas_err<S: sov_modules_api::Spec>(
    block_env: BlockEnv,
    mut tx_env: revm::primitives::TxEnv,
    cfg_env_with_handler: revm::primitives::CfgEnvWithHandlerCfg,
    db: EvmDb<'_, S>,
) -> EthApiError {
    let req_gas_limit = tx_env.gas_limit;
    tx_env.gas_limit = block_env.gas_limit;
    let res = executor::inspect(db, &block_env, tx_env, cfg_env_with_handler).unwrap();
    match res.result {
        ExecutionResult::Success { .. } => {
            // a transaction succeeded by manually increasing the gas limit to
            // highest, which means the caller lacks funds to pay for the tx
            RpcInvalidTransactionError::BasicOutOfGas(U256::from(req_gas_limit)).into()
        }
        ExecutionResult::Revert { output, .. } => {
            // reverted again after bumping the limit
            RpcInvalidTransactionError::Revert(RevertError::new(output.into())).into()
        }
        ExecutionResult::Halt { reason, .. } => RpcInvalidTransactionError::EvmHalt(reason).into(),
    }
}

fn convert_u256_to_u64(u256: U256) -> Result<u64, TryFromSliceError> {
    let bytes: [u8; 32] = u256.to_be_bytes();
    let bytes: [u8; 8] = bytes[24..].try_into()?;
    Ok(u64::from_be_bytes(bytes))
}
