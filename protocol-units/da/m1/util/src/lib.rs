use celestia_types::nmt::Namespace;
use celestia_rpc::Client;
use m1_da_light_node_grpc::*;

#[derive(Debug, Clone)]
pub struct Config {
    pub celestia_url : String,
    pub celestia_token : String,
    pub celestia_namespace : Namespace,
    pub verification_mode : VerificationMode
}

impl Config {

    const DEFAULT_CELESTIA_NODE_URL: &'static str = "ws://localhost:26658";
    const DEFAULT_NAMESPACE_BYTES: &'static str = "a673006fb64aa2e5360d";

    pub fn try_from_env() -> Result<Self, anyhow::Error> {
        let token = std::env::var("CELESTIA_NODE_AUTH_TOKEN").map_err(
            |_| anyhow::anyhow!("Token not provided")
        )?; // expect("Token not provided"
        let url = std::env::var("CELESTIA_NODE_URL").unwrap_or_else(|_| Self::DEFAULT_CELESTIA_NODE_URL.to_string());
        
        
        let namespace_hex = std::env::var("CELESTIA_NAMESPACE_BYTES")
        .unwrap_or_else(|_| Self::DEFAULT_NAMESPACE_BYTES.to_string());

        // Decode the hex string to bytes
        let namespace_bytes = hex::decode(namespace_hex).map_err(|e| anyhow::anyhow!("Failed to decode namespace bytes: {}", e))?;

        // Create a namespace from the bytes
        let namespace = Namespace::new_v0(&namespace_bytes)?;

         // try to read the verification mode from the environment
        let verification_mode = match std::env::var("VERIFICATION_MODE") {
            Ok(mode) => {
              VerificationMode::from_str_name(mode.as_str()).ok_or(anyhow::anyhow!("Invalid verification mode"))?
            },
            Err(_) => VerificationMode::MOfN
        };

        Ok(Self {
            celestia_url : url,
            celestia_token : token,
            celestia_namespace : namespace,
            verification_mode
        })

    }

    pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
        let client = Client::new(&self.celestia_url, Some(&self.celestia_token)).await.map_err(|e| anyhow::anyhow!("Failed to connect to Celestia client at {}: {}", self.celestia_url, e))?;
        Ok(client)
    }

}