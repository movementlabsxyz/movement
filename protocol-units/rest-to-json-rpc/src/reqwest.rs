use crate::{
    JsonRpcRequestStandard,
    JsonRpcRequest,
    Forwarder,
};

pub struct ReqwestForwarder {
    pub url: String,
}

#[async_trait::async_trait]
impl Forwarder for ReqwestForwarder {

    async fn forward(&self, json_rpc_request : JsonRpcRequest) -> Result<(), anyhow::Error> {
        let client = reqwest::Client::new();
        let response = client.post(&self.url)
        .json(&json_rpc_request)
        .send().await?;

    response.b

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to forward request"))
        }
    }

}