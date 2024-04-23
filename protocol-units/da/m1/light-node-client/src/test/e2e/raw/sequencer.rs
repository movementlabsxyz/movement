use anyhow::Ok;
use crate::*;
use tokio_stream::StreamExt;
use movement_types::Block;

#[tokio::test]
async fn test_light_node_submits_blob_over_stream() -> Result<(), anyhow::Error>{
    
    let mut client = LightNodeServiceClient::connect("http://[::1]:30730").await?;

    let stream = client.stream_read_latest(StreamReadLatestRequest {}).await?;

    let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let blob_write = BlobWrite {
        data : data.clone()
    };
    let batch_write_request = BatchWriteRequest {
        blobs : vec![blob_write.clone()]
    };
    client.batch_write(batch_write_request).await?;

    let back = stream.into_inner().next().await.ok_or(
        anyhow::anyhow!("No response from server")
    )??;

    match back.blob {
        Some(blob) => {
            let block = serde_json::from_slice::<Block>(&blob.data).map_err(
                |e| anyhow::anyhow!("Failed to deserialize block: {}", e)
            )?;
            assert_eq!(block.transactions[0].0, data);
        },
        None => {
            assert!(false, "No blob in response");
        }
    }
  

    Ok(())
}