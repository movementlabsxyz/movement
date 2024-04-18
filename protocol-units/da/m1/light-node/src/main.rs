use m1_da_light_node::v1::LightNodeV1;
use m1_da_light_node_grpc::light_node_service_server::LightNodeServiceServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(m1_da_light_node_grpc::FILE_DESCRIPTOR_SET)
        .build()?;

    let env_addr = std::env::var("M1_DA_LIGHT_NODE_ADDR").unwrap_or_else(|_| "[::1]:30730".to_string());
    let addr = env_addr.parse()?;
    let light_node = LightNodeV1::try_from_env().await?;

    Server::builder()
        .accept_http1(true)
        .add_service(LightNodeServiceServer::new(light_node))
        .add_service(reflection)
        .serve(addr)
        .await?;

    Ok(())
}