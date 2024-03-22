use std::convert::Infallible;

use reth_primitives::TransactionSignedEcRecovered;
use revm::primitives::{CfgEnvWithHandlerCfg, EVMError, Env, EnvWithHandlerCfg, ExecutionResult};
use revm::{self, Database, DatabaseCommit, EvmBuilder};

use super::primitive_types::BlockEnv;
use crate::evm::conversions::create_tx_env;

pub(crate) fn execute_tx<DB: Database<Error = Infallible> + DatabaseCommit>(
	db: DB,
	block_env: &BlockEnv,
	tx: &TransactionSignedEcRecovered,
	config_env: CfgEnvWithHandlerCfg,
) -> Result<ExecutionResult, EVMError<Infallible>> {
	todo!()
}

pub(crate) fn inspect<DB: Database<Error = Infallible> + DatabaseCommit>(
	db: DB,
	block_env: &BlockEnv,
	tx: revm::primitives::TxEnv,
	config_env: CfgEnvWithHandlerCfg,
) -> Result<revm::primitives::ResultAndState, EVMError<Infallible>> {
	todo!()
}
