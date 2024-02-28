use sov_rollup_interface::da::BlockHeaderTrait;
use sov_rollup_interface::{
    services::da::DaService,
    da::DaSpec,
};
use web3::ethabi::Token;
use web3::{
    contract::{Contract, Options},
    ethabi,
    types::{Address, Bytes, U256, H256},
    Web3,
};
use web3::transports::Http;
use web3::signing::Key;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::{any, fs};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct EthSettlementService<T: DaService<Error = anyhow::Error>> {
    pub da_service: T,
    pub web3_client: Arc<RwLock<Web3<Http>>>,
    pub contract_address: Address,
    pub contract: Contract<Http>,
}

impl<T: DaService<Error = anyhow::Error>> EthSettlementService<T> {

    /// Attempts to create a new `EthSettlementService` with the provided RPC URL and contract address.
    pub fn try_new(
        da_service: T, 
        rpc_url: &str, 
        contract_address: &str,
        contract_path : &str
    ) -> Result<Self, web3::Error> {
        
        let http = Http::new(rpc_url)?;
        let web3_client = Web3::new(http);
        let contract_address = contract_address.parse().map_err(|_| web3::Error::Decoder("Failed to parse contract address".into()))?;

        let file_contents = fs::read_to_string(contract_path)?;

        // Parse the string into a JSON object
        let json: Value = serde_json::from_str(&file_contents)?;

        // Extract the ABI part of the JSON
        let abi = json["abi"].clone();

        let contract = Contract::from_json(
            web3_client.eth(),
            contract_address,
            // read the bytes from the contract path
            abi.to_string().as_bytes()
        ).map_err(
            |e| web3::Error::Decoder(format!("Failed to create contract instance: {}", e))
        )?;

        Ok(Self {
            da_service,
            web3_client: Arc::new(RwLock::new(web3_client)),
            contract_address,
            contract,
        })
    }

    /// Attempts to create a new `EthSettlementService` using an RPC URL from the environment.
    pub fn try_env(da_service: T) -> Result<Self, web3::Error> {
        
        let rpc_url = std::env::var("ETH_RPC_URL").map_err(|_| web3::Error::Transport(
            web3::error::TransportError::Message(
                String::from("ETH_RPC_URL environment variable not set")
            )
        ))?;

        let contract_address = std::env::var("ETH_CONTRACT_ADDRESS").map_err(|_| web3::Error::Transport(
            web3::error::TransportError::Message(
                String::from("ETH_CONTRACT_ADDRESS environment variable not set")
            )
        ))?;

        let contract_path = std::env::var("ETH_CONTRACT_ABI_PATH").map_err(|_| web3::Error::Transport(
            web3::error::TransportError::Message(
                String::from("ETH_CONTRACT_ABI_PATH environment variable not set")
            )
        ))?;

        Self::try_new(
            da_service, &rpc_url, 
            &contract_address, contract_path.as_str()
        )
    }


}


#[async_trait::async_trait]
impl<T: DaService<Error = anyhow::Error>> DaService for EthSettlementService<T> {
   
    type Error = T::Error;
    type FilteredBlock = T::FilteredBlock;
    type Spec = T::Spec;
    type Verifier = T::Verifier;
    type HeaderStream = T::HeaderStream;
    type TransactionId = T::TransactionId;

    async fn get_block_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        self.da_service.get_block_at(height).await
    }

    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        self.da_service.get_last_finalized_block_header().await
    }

    async fn subscribe_finalized_header(&self) -> Result<Self::HeaderStream, Self::Error> {
        self.da_service.subscribe_finalized_header().await
    }

    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        self.da_service.get_head_block_header().await
    }

    fn extract_relevant_blobs(
        &self,
        block: &Self::FilteredBlock,
    ) -> Vec<<Self::Spec as DaSpec>::BlobTransaction> {
        self.da_service.extract_relevant_blobs(block)
    }

    async fn get_extraction_proof(
        &self,
        block: &Self::FilteredBlock,
        blobs: &[<Self::Spec as DaSpec>::BlobTransaction],
    ) -> (
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    ) {
        self.da_service.get_extraction_proof(block, blobs).await
    }

 
    async fn send_transaction(&self, blob: &[u8]) -> Result<Self::TransactionId, Self::Error> {
        self.da_service.send_transaction(blob).await
    }

    // Sends an aggregated ZK proof to the Ethereum blockchain
    async fn send_aggregated_zk_proof(&self, aggregated_proof_data: &[u8]) -> Result<u64, Self::Error> {

        // todo: this is too naive, but for now we just use the last finalized block height
        let height = self.get_last_finalized_block_header().await?.height();

        let web3 = self.web3_client.read().await;
        let accounts = web3.eth().accounts().await?;
        let from = accounts[0]; // Ensure this account is unlocked and has enough balance.

        let proof_data = Bytes(aggregated_proof_data.to_vec());

        let tx_hash = self.contract
            .call("settle", (height, proof_data), from, Options::default())
            .await?;
        
     
        Ok(height)
    }


    async fn get_aggregated_proofs_at(&self, height: u64) -> Result<Vec<Vec<u8>>, Self::Error> {

        let block_height = U256::from(height);

        let proofs: Vec<Vec<u8>> = self.contract
            .query("getProofsAtHeight", block_height, None, Options::default(), None)
            .await?;
    
        Ok(proofs)

    }
    
}

#[cfg(test)]
pub mod test {

    use super::*;
    use sov_mock_da::{MockDaService, MockAddress};

    #[tokio::test]
    async fn test_eth_settlement_service_env()-> Result<(), anyhow::Error> {
        
        let da_service = MockDaService::new(
            // 32 &[u8] bytes
            MockAddress::new([0; 32]),
        );
        
        let da = EthSettlementService::try_env(da_service)?;

        da.send_aggregated_zk_proof(&[0; 32]).await?;
        let proofs = da.get_aggregated_proofs_at(0).await?;
        assert_eq!(proofs[0], [0; 32]);

        Ok(())
    }

}