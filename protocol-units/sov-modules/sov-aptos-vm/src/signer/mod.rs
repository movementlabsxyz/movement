use aptos_crypto::hash::CryptoHash;
use aptos_crypto::{bls12381, CryptoMaterialError, SigningKey};
use aptos_types::account_address::AccountAddress;
use std::collections::HashMap;

use reth_primitives::{sign_message, TransactionSigned};
use reth_rpc_types::TypedTransactionRequest;
use reth_rpc_types_compat::transaction::to_primitive_transaction;
use revm::primitives::{Address, B256};
use secp256k1::{PublicKey, SecretKey};
use serde::ser::Serialize;

/// Ethereum transaction signer.
#[derive(Clone)]
pub struct DevSigner {
	signers: HashMap<AccountAddress, bls12381::PrivateKey>,
}

impl DevSigner {
	/// Creates a new DevSigner.
	pub fn new(keys: Vec<(AccountAddress, bls12381::PrivateKey)>) -> Self {
		let mut signers = HashMap::with_capacity(keys.len());
		for key in keys {
			let (public, private) = key;
			signers.insert(public, private);
		}
		Self { signers }
	}

	/// Signs an ethereum transaction.
	pub fn sign_transaction<T: Serialize + CryptoHash>(
		&self,
		message: &T,
		address: &AccountAddress,
	) -> Result<bls12381::Signature, CryptoMaterialError> {
		let signer = self.signers.get(address)?;
		signer.sign(message)
	}

	/// List of signers.
	pub fn signers(&self) -> Vec<AccountAddress> {
		self.signers.keys().copied().collect()
	}
}
