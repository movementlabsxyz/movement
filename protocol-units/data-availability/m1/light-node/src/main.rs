use m1_da_light_node::v1::LightNodeV1;
use m1_da_light_node_grpc::light_node_server::LightNodeServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(m1_da_light_node_grpc::FILE_DESCRIPTOR_SET)
        .build()?;


    let addr = "[::1]:30730".parse()?;
    let light_node = LightNodeV1 {};

    Server::builder()
        .accept_http1(true)
        .add_service(reflection)
        .add_service(LightNodeServer::new(light_node))
        .serve(addr)
        .await?;

    Ok(())
}