use std::{str::FromStr, time::Duration};
use anyhow::{anyhow, Context, Error, Result};
use ethers::{
    core::k256::{ecdsa::SigningKey, SecretKey},
    middleware::SignerMiddleware,
    prelude::*,
    providers::{Provider, Ws},
};
use risc0_ethereum_relay::EthersClientConfig as Risc0EthersClientConfig;
use tracing::{debug, error};

/// We use this in combination with `EthersClientConfig` to 
/// create the client. An example of this can be found in tests/integration/src/lib.rs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletKey(SecretKey);

impl TryFrom<String> for WalletKey {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let decoded =
            hex::decode(value.trim_start_matches("0x")).context("Failed to decode private key.")?;
        let key =
            SecretKey::from_slice(&decoded).context("Failed to derive SecretKey instance.")?;
        Ok(Self(key))
    }
}

impl FromStr for WalletKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_string().try_into()
    }
}

impl From<SecretKey> for WalletKey {
    fn from(value: SecretKey) -> Self {
        Self(value)
    }
}

impl From<WalletKey> for SecretKey {
    fn from(value: WalletKey) -> Self {
        value.0
    }
}

impl WalletKey {
    pub fn get_key(&self) -> SecretKey {
        self.0.clone()
    }
}

type EthersClientConfig = Risc0EthersClientConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EthersClientConfig {
    pub eth_node_url: String,
    pub eth_chain_id: u64,
    pub wallet_key_identifier: WalletKey,
    pub retries: u64,
    pub wait_time: Duration,
}

impl EthersClientConfig {
    pub fn new(
        eth_node_url: String,
        eth_chain_id: u64,
        wallet_key_identifier: WalletKey,
        retries: u64,
        wait_time: Duration,
    ) -> Self {
        Self {
            eth_node_url,
            eth_chain_id,
            wallet_key_identifier,
            retries,
            wait_time,
        }
    }

    pub async fn get_client(&self) -> Result<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>> {
        let provider = self.provider().await?;
        let signer = self.get_signer()?;
        let client = SignerMiddleware::new(provider, signer);
        Ok(client)
    }

    pub async fn provider(&self) -> Result<Provider<Ws>> {
        let provider = Provider::<Ws>::connect_with_reconnects(self.eth_node_url.clone(), 60)
            .await
            .context("Failed to connect to Ethereum node.")?;
        Ok(provider)
    }

    pub fn get_signer(&self) -> Result<Wallet<SigningKey>> {
        let signing_key = SigningKey::from(self.wallet_key_identifier.get_key());
        let signer = LocalWallet::from(signing_key).with_chain_id(self.eth_chain_id);
        Ok(signer)
    }

    pub async fn get_client_with_reconnects(
        &self,
    ) -> Result<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>> {
        for _ in 0..self.retries {
            let client = self.get_client().await;
            if client.is_ok() {
                return client;
            } else {
                debug!(
                    "Failed to create client. Retrying in {:?} seconds.",
                    self.wait_time.as_secs()
                );
                tokio::time::sleep(self.wait_time).await;
            }
        }
        error!("Failed to create client.");
        Err(anyhow!("Failed to create client."))
    }
}