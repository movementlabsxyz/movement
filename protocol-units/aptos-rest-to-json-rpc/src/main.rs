use aptos_rest_to_json_rpc::v1;
use rest_to_json_rpc::Proxy;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    
    let server = v1::V1Proxy::try_actix()?;

    server.serve().await?;

    Ok(())

}