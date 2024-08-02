use anyhow::Context;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tokio::task;
use alloy::{pubsub::PubSubFrontend, signers::local::PrivateKeySigner};
use alloy_network::EthereumWallet;
use alloy_primitives::{
    private::serde::{Deserialize, Serialize},
    Address, FixedBytes, U256,
};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_rlp::{Decodable, RlpDecodable, RlpEncodable};
use alloy_sol_types::sol;
use bridge_shared::bridge_contracts::{
    BridgeContractCounterparty, BridgeContractCounterpartyError, BridgeContractCounterpartyResult,
    BridgeContractInitiator, BridgeContractInitiatorError, BridgeContractInitiatorResult,
};
use bridge_shared::types::{
    Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
    RecipientAddress, TimeLock,
};

pub mod types;
pub mod utils;

use crate::types::{EthAddress, EthHash, DEFAULT_GAS_LIMIT, INITIATOR_CONTRACT};

// Codegen from the abis
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    AtomicBridgeInitiator,
    "abis/AtomicBridgeInitiator.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    AtomicBridgeCounterparty,
    "abis/AtomicBridgeCounterparty.json"
);

/// Configuration for the Ethereum Bridge Client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: Option<String>,
    pub ws_url: Option<String>,
    pub chain_id: String,
    pub signer_private_key: String,
    pub initiator_contract: Option<EthAddress>,
    pub counterparty_contract: Option<EthAddress>,
    pub gas_limit: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rpc_url: Some("http://localhost:8545".to_string()),
            ws_url: Some("ws://localhost:8545".to_string()),
            chain_id: "31337".to_string(),
            signer_private_key: Self::default_for_private_key(),
            initiator_contract: Some(EthAddress::from(INITIATOR_CONTRACT.to_string())),
            counterparty_contract: None,
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }
}

impl Config {
    fn default_for_private_key() -> String {
        let random_wallet = PrivateKeySigner::random();
        random_wallet.to_bytes().to_string()
    }
}

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetails {
    pub amount: U256,
    pub originator: EthAddress,
    pub recipient: [u8; 32],
    pub hash_lock: [u8; 32],
    pub time_lock: U256,
    pub state: u8,
}

/// Connection enum to represent either an RPC or WebSocket connection
enum Connection<P> {
    Rpc(P),
    Ws(RootProvider<PubSubFrontend>),
}

pub struct EthClient<P> {
    rpc_providers: Arc<Mutex<Vec<P>>>,
    ws_providers: Arc<Mutex<Vec<RootProvider<PubSubFrontend>>>>,
    initiator_contract: Option<EthAddress>,
    counterparty_contract: Option<EthAddress>,
    max_connections: usize,
}

impl<P> EthClient<P>
where
    P: Send + Sync + 'static,
{
    /// Creates a new Ethereum client with a specified maximum number of connections
    pub fn new(
        rpc_providers: Vec<P>,
        ws_providers: Vec<RootProvider<PubSubFrontend>>,
        initiator_contract: Option<EthAddress>,
        counterparty_contract: Option<EthAddress>,
        max_connections: usize,
    ) -> Self {
        Self {
            rpc_providers: Arc::new(Mutex::new(rpc_providers)),
            ws_providers: Arc::new(Mutex::new(ws_providers)),
            initiator_contract,
            counterparty_contract,
            max_connections,
        }
    }

    /// Adds an RPC provider to the pool
    fn add_rpc_provider(&self, provider: P) {
        let mut rpc_providers = self.rpc_providers.lock().unwrap();
        if rpc_providers.len() < self.max_connections {
            rpc_providers.push(provider);
        } else {
            println!("Max RPC connections reached");
        }
    }

    /// Adds a WebSocket provider to the pool
    fn add_ws_provider(&self, provider: RootProvider<PubSubFrontend>) {
        let mut ws_providers = self.ws_providers.lock().unwrap();
        if ws_providers.len() < self.max_connections {
            ws_providers.push(provider);
        } else {
            println!("Max WebSocket connections reached");
        }
    }

    /// Fetches an RPC connection from the pool, or creates a new one if possible
    async fn get_rpc_connection(&self, rpc_url: &str, signer: &PrivateKeySigner) -> Option<P>
    where
        P: Provider + Clone + Debug,
    {
        {
            let mut rpc_providers = self.rpc_providers.lock().unwrap();
            if let Some(provider) = rpc_providers.pop() {
                return Some(provider);
            }
        }

        if self.rpc_providers.lock().unwrap().len() < self.max_connections {
            let provider = ProviderBuilder::new()
                .wallet(EthereumWallet::from(signer.clone()))
                .on_builtin(rpc_url)
                .await
                .ok()?;
            self.add_rpc_provider(provider.clone());
            Some(provider)
        } else {
            None
        }
    }

    /// Fetches a WebSocket connection from the pool, or creates a new one if possible
    async fn get_ws_connection(&self, ws_url: &str) -> Option<RootProvider<PubSubFrontend>> {
        {
            let mut ws_providers = self.ws_providers.lock().unwrap();
            if let Some(provider) = ws_providers.pop() {
                return Some(provider);
            }
        }

        if self.ws_providers.lock().unwrap().len() < self.max_connections {
            let ws = WsConnect::new(ws_url.to_string());
            let provider = ProviderBuilder::new().on_ws(ws).await.ok()?;
            self.add_ws_provider(provider.clone());
            Some(provider)
        } else {
            None
        }
    }

    /// Executes an operation asynchronously on a connection
    pub async fn execute<F, T>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce(Connection<P>) -> Result<T, anyhow::Error> + Send + 'static,
        T: Send + 'static,
    {
        let rpc_providers = self.rpc_providers.clone();
        let ws_providers = self.ws_providers.clone();

        task::spawn_blocking(move || {
            let rpc_providers = rpc_providers.lock().unwrap();
            let ws_providers = ws_providers.lock().unwrap();

            if let Some(rpc_provider) = rpc_providers.last() {
                f(Connection::Rpc(rpc_provider.clone()))
            } else if let Some(ws_provider) = ws_providers.last() {
                f(Connection::Ws(ws_provider.clone()))
            } else {
                Err(anyhow::Error::msg("No available connections"))
            }
        })
        .await?
    }
}

// Implement BridgeContractInitiator and BridgeContractCounterparty traits here

impl EthClient<utils::AlloyProvider> {
    fn initiator_contract(&self) -> BridgeContractInitiatorResult<Address> {
        match &self.initiator_contract {
            Some(address) => Ok(address.0),
            None => Err(BridgeContractInitiatorError::InitiatorAddressNotSet),
        }
    }

    fn counterparty_contract(&self) -> BridgeContractCounterpartyResult<Address> {
        match &self.counterparty_contract {
            Some(address) => Ok(address.0),
            None => Err(BridgeContractCounterpartyError::CounterpartyAddressNotSet),
        }
    }
}

impl Clone for EthClient<utils::AlloyProvider> {
    fn clone(&self) -> Self {
        Self {
            rpc_providers: Arc::clone(&self.rpc_providers),
            ws_providers: Arc::clone(&self.ws_providers),
            initiator_contract: self.initiator_contract.clone(),
            counterparty_contract: self.counterparty_contract.clone(),
            max_connections: self.max_connections,
        }
    }
}