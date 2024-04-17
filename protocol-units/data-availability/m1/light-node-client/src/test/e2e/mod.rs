use anyhow::Ok;
use m1_da_light_node_grpc::light_node_client::LightNodeClient;
use m1_da_light_node_grpc::*;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_light_node_submits_blob_over_stream() -> Result<(), anyhow::Error>{
    
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

    let back = handle.into_inner().next().await.ok_or(
        anyhow::anyhow!("No response from server")
    )??;

    match back.blob {
        Some(blob) => {
            assert_eq!(blob.data, request.data);
        },
        None => {
            assert!(false, "No blob in response");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_submit_and_read() -> Result<(), anyhow::Error>{
    
    let mut client = LightNodeClient::connect("http://[::1]:30730").await?;

    let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let request = BatchWriteRequest {
        data : vec![data.clone()]
    };

    let write = client.batch_write(request).await?;

    let read_request = ReadAtHeightRequest {
        height : write.into_inner().blobs[0].height,
    };

    let read = client.read_at_height(read_request).await?;

    assert_eq!(read.into_inner().blobs[0].data, data);

    Ok(())

}