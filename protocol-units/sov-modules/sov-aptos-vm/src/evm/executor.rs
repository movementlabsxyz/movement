use std::convert::Infallible;

use reth_primitives::TransactionSignedEcRecovered;
use revm::primitives::{CfgEnvWithHandlerCfg, EVMError, ExecutionResult};
use revm::{self, Database, DatabaseCommit};

use super::primitive_types::BlockEnv;

pub(crate) fn execute_tx<DB: Database<Error = Infallible> + DatabaseCommit>(
	_db: DB,
	_block_env: &BlockEnv,
	_tx: &TransactionSignedEcRecovered,
	_config_env: CfgEnvWithHandlerCfg,
) -> Result<ExecutionResult, EVMError<Infallible>> {
	todo!()
}

pub(crate) fn inspect<DB: Database<Error = Infallible> + DatabaseCommit>(
	_db: DB,
	_block_env: &BlockEnv,
	_tx: revm::primitives::TxEnv,
	_config_env: CfgEnvWithHandlerCfg,
) -> Result<revm::primitives::ResultAndState, EVMError<Infallible>> {
	todo!()
}
