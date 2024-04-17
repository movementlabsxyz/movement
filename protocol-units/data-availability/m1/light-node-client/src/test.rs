use m1_da_light_node_grpc::light_node_client::LightNodeClient;
use m1_da_light_node_grpc::*;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_light_node_v1() -> Result<(), anyhow::Error>{
    
    let mut client = LightNodeClient::connect("http://[::1]:30730").await?;

    let request = BlobWriteRequest {
        data : vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    };

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    // Convert the receiver into a stream
    let stream = ReceiverStream::new(rx);

    let handle = client.stream_write_blob(
        stream
    ).await?;

    tx.send(request.clone()).await?;

    handle.into_inner().next().await.unwrap()?;

    println!("Sent messages.");

    Ok(())
}