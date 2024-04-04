use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    JsonRpcRequest,
    Forwarder,
};
use reqwest::Response;

#[derive(Debug, Clone)]
pub struct ReqwestForwarder {
    pub url: Arc<RwLock<String>>,
}

#[async_trait::async_trait]
impl Forwarder<Response> for ReqwestForwarder {

    async fn forward(&self, json_rpc_request : JsonRpcRequest) -> Result<Response, anyhow::Error> {
        let url = self.url.read().await;
        let client = reqwest::Client::new();
        let response = client.post(&*url)
        .json(&json_rpc_request)
        .send().await?;

        Ok(response)

    }

}