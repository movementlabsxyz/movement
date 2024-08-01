use async_trait::async_trait;
use anyhow::{Context, Error};
use std::{sync::Arc, fmt::Debug};

use alloy::{pubsub::PubSubFrontend, signers::local::PrivateKeySigner};
use alloy_network::EthereumWallet;
use alloy_primitives::{
    private::serde::{Deserialize, Serialize},
    Address, FixedBytes, U256,
};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_rlp::{Decodable, RlpDecodable, RlpEncodable};
use alloy_sol_types::sol;
use alloy_transport::{Transport, BoxTransport};
use deadpool::managed::{Manager, Pool, RecycleError};
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

/// Connection Manager for the Deadpool
pub struct ConnectionManager {
    rpc_url: Arc<str>,
    signer: Arc<PrivateKeySigner>,
}

impl ConnectionManager {
    pub fn new(rpc_url: Arc<str>, signer: Arc<PrivateKeySigner>) -> Self {
        Self { rpc_url, signer }
    }
}

#[async_trait]
impl Manager for ConnectionManager {
        type Type = RootProvider<BoxTransport>;
        type Error = anyhow::Error;

        async fn create(&self) -> Result<Self::Type, Self::Error> {
                // Create a WsConnect instance to manage the connection
                let transport = WsConnect::new(self.rpc_url.clone())
                        .await
                        .context("Failed to connect to WebSocket")?;

                // Convert WsConnect into a Transport using the boxed method
                let boxed_transport = transport.boxed();

                // Build the provider using the boxed transport
                let provider: RootProvider<BoxTransport> = ProviderBuilder::new()
                        .wallet(EthereumWallet::from(self.signer.clone()))
                        .on_provider(boxed_transport);

                Ok(provider)
        }

        async fn recycle(&self, _conn: &mut Self::Type) -> Result<(), RecycleError<Self::Error>> {
                Ok(())
        }
}

/// Ethereum Client
pub struct EthClient {
    pool: Pool<ConnectionManager>,
    ws_provider: RootProvider<PubSubFrontend>,
    initiator_contract: Option<EthAddress>,
    counterparty_contract: Option<EthAddress>,
}

impl EthClient {
    pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
        let signer = config.signer_private_key.parse::<PrivateKeySigner>()?;
        let rpc_url = config.rpc_url.context("rpc_url not set")?;
        let ws_url = config.ws_url.context("ws_url not set")?;

        let manager = ConnectionManager::new(Arc::from(rpc_url), Arc::new(signer));
        let pool = Pool::builder(manager)
            .max_size(32)
            .build()
            .await?;

        let ws = WsConnect::new(ws_url);
        let ws_provider = ProviderBuilder::new().on_ws(ws).await?;
        Ok(EthClient {
            pool,
            ws_provider,
            initiator_contract: config.initiator_contract,
            counterparty_contract: config.counterparty_contract,
        })
    }
}

#[async_trait::async_trait]
impl BridgeContractInitiator for EthClient {
    type Address = EthAddress;
    type Hash = EthHash;

    async fn initiate_bridge_transfer(
        &mut self,
        _initiator_address: InitiatorAddress<Self::Address>,
        recipient_address: RecipientAddress<Vec<u8>>,
        hash_lock: HashLock<Self::Hash>,
        time_lock: TimeLock,
        amount: Amount,
    ) -> BridgeContractInitiatorResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractInitiatorError::GenericError(format!("{:?}", e)))?;

        let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &*conn);
        let recipient_bytes: [u8; 32] = recipient_address.0.try_into().unwrap();
        let call = contract.initiateBridgeTransfer(
            U256::from(amount.0),
            FixedBytes(recipient_bytes),
            FixedBytes(hash_lock.0),
            U256::from(time_lock.0),
        );

        utils::send_transaction(call)
            .await
            .map_err(BridgeContractInitiatorError::generic)
            .map(|_| ())
    }

    async fn complete_bridge_transfer(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
        pre_image: HashLockPreImage,
    ) -> BridgeContractInitiatorResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractInitiatorError::GenericError(format!("{:?}", e)))?;

        let generic_error = |desc| BridgeContractInitiatorError::GenericError(String::from(desc));
        let pre_image: [u8; 32] = pre_image
            .0
            .get(0..32)
            .ok_or(generic_error("Could not get required slice from pre-image"))?
            .try_into()
            .map_err(|_| generic_error("Could not convert pre-image to [u8; 32]"))?;

        let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &*conn);
        let call =
            contract.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));

        utils::send_transaction(call)
            .await
            .map_err(BridgeContractInitiatorError::generic)
            .map(|_| ())
    }

    async fn refund_bridge_transfer(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
    ) -> BridgeContractInitiatorResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractInitiatorError::GenericError(format!("{:?}", e)))?;

        let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &*conn);
        let call = contract.refundBridgeTransfer(FixedBytes(bridge_transfer_id.0));

        utils::send_transaction(call)
            .await
            .map_err(BridgeContractInitiatorError::generic)
            .map(|_| ())
    }

    async fn get_bridge_transfer_details(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
    ) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
        let generic_error = |desc| BridgeContractInitiatorError::GenericError(String::from(desc));
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractInitiatorError::GenericError(format!("{:?}", e)))?;

        let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
        let key = bridge_transfer_id.0;
        let storage_slot = utils::calculate_storage_slot(key, mapping_slot);
        let storage: U256 = conn
            .get_storage_at(self.initiator_contract()?, storage_slot)
            .await
            .map_err(|_| generic_error("could not find storage"))?;
        let storage_bytes = storage.to_be_bytes::<32>();
        let mut storage_slice = &storage_bytes[..];
        let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
            .map_err(|_| generic_error("could not decode storage"))?;

        Ok(Some(BridgeTransferDetails {
            bridge_transfer_id,
            initiator_address: InitiatorAddress(eth_details.originator),
            recipient_address: RecipientAddress(eth_details.recipient.to_vec()),
            hash_lock: HashLock(eth_details.hash_lock),
            //@TODO unit test these wrapping to check for any nasty side effects.
            time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
            amount: Amount(eth_details.amount.wrapping_to::<u64>()),
        }))
    }
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for EthClient {
    type Address = EthAddress;
    type Hash = EthHash;

    async fn lock_bridge_transfer_assets(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
        hash_lock: HashLock<Self::Hash>,
        time_lock: TimeLock,
        initiator: InitiatorAddress<Vec<u8>>,
        recipient: RecipientAddress<Self::Address>,
        amount: Amount,
    ) -> BridgeContractCounterpartyResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractCounterpartyError::GenericError(format!("{:?}", e)))?;

        let contract =
            AtomicBridgeCounterparty::new(self.counterparty_contract()?, &*conn);
        let initiator: [u8; 32] = initiator.0.try_into().unwrap();
        let call = contract.lockBridgeTransferAssets(
            FixedBytes(initiator),
            FixedBytes(bridge_transfer_id.0),
            FixedBytes(hash_lock.0),
            U256::from(time_lock.0),
            Address::from(recipient.0 .0),
            U256::from(amount.0),
        );
        utils::send_transaction(call)
            .await
            .map_err(BridgeContractCounterpartyError::generic)
            .map(|_| ())
    }

    async fn complete_bridge_transfer(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
        secret: HashLockPreImage,
    ) -> BridgeContractCounterpartyResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractCounterpartyError::GenericError(format!("{:?}", e)))?;

        let contract =
            AtomicBridgeCounterparty::new(self.counterparty_contract()?, &*conn);
        let secret: [u8; 32] = secret.0.try_into().unwrap();
        let call =
            contract.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(secret));
        utils::send_transaction(call)
            .await
            .map_err(BridgeContractCounterpartyError::generic)
            .map(|_| ())
    }

    async fn abort_bridge_transfer(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
    ) -> BridgeContractCounterpartyResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractCounterpartyError::GenericError(format!("{:?}", e)))?;

        let contract =
            AtomicBridgeCounterparty::new(self.counterparty_contract()?, &*conn);
        let call = contract.abortBridgeTransfer(FixedBytes(bridge_transfer_id.0));
        utils::send_transaction(call)
            .await
            .map_err(BridgeContractCounterpartyError::generic)
            .map(|_| ())
    }

    async fn get_bridge_transfer_details(
        &mut self,
        bridge_transfer_id: BridgeTransferId<Self::Hash>,
    ) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
        let generic_error =
            |desc| BridgeContractCounterpartyError::GenericError(String::from(desc));
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BridgeContractCounterpartyError::GenericError(format!("{:?}", e)))?;

        let mapping_slot = U256::from(1); // the mapping is the 1st slot in the contract
        let key = bridge_transfer_id.0;
        let storage_slot = utils::calculate_storage_slot(key, mapping_slot);
        let storage: U256 = conn
            .get_storage_at(self.counterparty_contract()?, storage_slot)
            .await
            .map_err(|_| generic_error("could not find storage"))?;
        let storage_bytes = storage.to_be_bytes::<32>();
        let mut storage_slice = &storage_bytes[..];
        let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
            .map_err(|_| generic_error("could not decode storage"))?;

        Ok(Some(BridgeTransferDetails {
            bridge_transfer_id,
            initiator_address: InitiatorAddress(eth_details.originator),
            recipient_address: RecipientAddress(eth_details.recipient.to_vec()),
            hash_lock: HashLock(eth_details.hash_lock),
            //@TODO unit test these wrapping to check for any nasty side effects.
            time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
            amount: Amount(eth_details.amount.wrapping_to::<u64>()),
        }))
    }
}

impl EthClient {
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

// See tracking issue: https://github.com/movementlabsxyz/movement/issues/250
impl Clone for EthClient {
    fn clone(&self) -> Self {
        EthClient {
            pool: self.pool.clone(),
            ws_provider: self.ws_provider.clone(),
            initiator_contract: self.initiator_contract.clone(),
            counterparty_contract: self.counterparty_contract.clone(),
        }
    }
}
