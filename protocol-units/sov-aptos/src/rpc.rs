use crate::DevSigner;
use std::array::TryFromSliceError;
use crate::experimental::SovAptosVM;
use aptos_api_types::{Address, MoveModuleBytecode, MoveResource, U64};
use aptos_crypto::bls12381::Signature;
use aptos_types::state_store::state_value::StateValue as AptosStateValue;
use aptos_types::transaction::Version;
use jsonrpsee::core::RpcResult;
use reth_primitives::{TransactionSignedEcRecovered, U128};
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::{
	CryptoSpec, DaSpec, StateMap, StateMapAccessor, StateValueAccessor, StateVecAccessor,
	WorkingSet,
};
use tracing::debug;

#[derive(Clone)]
pub struct EthRpcConfig<S: sov_modules_api::Spec> {

}

#[rpc_gen(client, server)]
impl<S: sov_modules_api::Spec> SovAptosVM<S> {
	
}
