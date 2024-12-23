use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, Address, ChainId, B256};
use alloy_signer::{sign_transaction_with_chain_id, Result, Signature as AlloySignature, Signer};
use k256::ecdsa::{self, VerifyingKey};
use signer::{
    cryptography::secp256k1::Secp256k1, Bytes, SignerOperations, SignerError, Signature as MvtSignature, PublicKey};
use std::fmt;

pub struct HsmSigner {
    kms: Box<dyn SignerOperations<Secp256k1>+ Sync + Send> ,
    pubkey: VerifyingKey,
    address: Address,
    chain_id: Option<ChainId>,
}

impl fmt::Debug for HsmSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HsmSigner")
            .field("chain_id", &self.chain_id)
            .field("pubkey", &hex::encode(self.pubkey.to_sec1_bytes()))
            .field("address", &self.address)
            .finish()
    }
}

#[async_trait::async_trait]
impl alloy_network::TxSigner<AlloySignature> for HsmSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<AlloySignature>,
    ) -> Result<AlloySignature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_hash(&tx.signature_hash()).await)
    }
}

#[async_trait::async_trait]
impl Signer for HsmSigner {
    async fn sign_hash(&self, hash: &B256) -> Result<AlloySignature> {
        self.sign_digest(hash).await.map(|sign| sign.into()).map_err(alloy_signer::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

impl HsmSigner {
    /// Instantiate a new signer from an existing `Client` and key ID.
    ///
    /// Retrieves the public key from HMS and calculates the Ethereum address.
    pub async fn new(
        kms: Box<dyn SignerOperations<Secp256k1> + Sync+ Send>,
        chain_id: Option<ChainId>,
    ) -> Result<HsmSigner, SignerError> {
        let resp = request_get_pubkey(&*kms).await?;
        let pubkey = decode_pubkey(resp)?;
        let address = alloy_signer::utils::public_key_to_address(&pubkey);
        Ok(Self { kms, chain_id, pubkey, address })
    }

    /// Fetch the pubkey associated with this signer's key ID.
    pub async fn get_pubkey(&self) -> Result<VerifyingKey, SignerError> {
        request_get_pubkey(&*self.kms).await.and_then(decode_pubkey)
    }

    /// Sign a digest with this signer's key and applies EIP-155.
    pub async fn sign_digest(&self, digest: &B256) -> Result<AlloySignature, SignerError> {
        let sig = request_sign_digest(&*self.kms, digest).await.and_then(decode_signature)?;
        println!("MVT AWAS SIGN: {}", hex::encode(sig.to_bytes()));
        let mut sig = sig_from_digest_bytes_trial_recovery(sig, digest, &self.pubkey);
        if let Some(chain_id) = self.chain_id {
            sig = sig.with_chain_id(chain_id);
        }
        Ok(sig)
    }
}

async fn request_get_pubkey(
    kms: &dyn SignerOperations<Secp256k1>,
) -> Result<PublicKey, SignerError> {
    kms.public_key().await
}

async fn request_sign_digest(
    kms: &(dyn SignerOperations<Secp256k1> + Sync),
    digest: &B256,
) -> Result<MvtSignature, SignerError> {
    kms.sign(Bytes(digest.as_slice().to_vec())).await
}

/// Decode an AWS KMS Pubkey response.
fn decode_pubkey(pk: PublicKey) -> Result<VerifyingKey, SignerError> {
    let pk_ref: &[u8] = &pk.0.0;
    let spki = spki::SubjectPublicKeyInfoRef::try_from(pk_ref).map_err(|err| SignerError::PublicKey(err.to_string()))?;
    let key = VerifyingKey::from_sec1_bytes(spki.subject_public_key.raw_bytes()).map_err(|err| SignerError::Sign(err.to_string()))?;
    Ok(key)
}

/// Decode an AWS KMS Signature response.
fn decode_signature(sign: MvtSignature) -> Result<ecdsa::Signature, SignerError> {
    let sig = ecdsa::Signature::from_der(&sign.0.0).map_err(|err| SignerError::Sign(err.to_string()))?;
    Ok(sig.normalize_s().unwrap_or(sig))
}

/// Recover an rsig from a signature under a known key by trial/error.
fn sig_from_digest_bytes_trial_recovery(
    sig: ecdsa::Signature,
    hash: &B256,
    pubkey: &VerifyingKey,
) -> AlloySignature {
    let signature = AlloySignature::from_signature_and_parity(sig, false).unwrap();
    if check_candidate(&signature, hash, pubkey) {
        return signature;
    }

    let signature = signature.with_parity(true);
    if check_candidate(&signature, hash, pubkey) {
        return signature;
    }

    panic!("bad sig");
}

/// Makes a trial recovery to check whether an RSig corresponds to a known `VerifyingKey`.
fn check_candidate(signature: &AlloySignature, hash: &B256, pubkey: &VerifyingKey) -> bool {
    signature.recover_from_prehash(hash).map(|key| key == *pubkey).unwrap_or(false)
}