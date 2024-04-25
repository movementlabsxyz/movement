use crate::*;
use tokio_stream::StreamExt;
use movement_types::Block;

#[tokio::test]
async fn test_light_node_submits_blob_over_stream() -> Result<(), anyhow::Error>{
    
    let mut client = LightNodeServiceClient::connect("http://[::1]:30730").await?;

    let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let blob_write = BlobWrite {
        data : data.clone()
    };
    let batch_write_request = BatchWriteRequest {
        blobs : vec![blob_write.clone()]
    };
    client.batch_write(batch_write_request).await?;

    let mut log_lines = Vec::new();

    for _ in 0..16 {

        let stream = client.stream_read_latest(StreamReadLatestRequest {}).await?;

        let back = stream.into_inner().next().await.ok_or(
            anyhow::anyhow!("No response from server")
        )?;

        match back {
            Ok(response) => {
                match response.blob {
                    Some(blob) => {
                        match blob.blob_type.ok_or(
                            anyhow::anyhow!("No blob type in response")
                        )? {
                            blob_response::BlobType::SequencedBlobBlock(blob) => {
                                let block = serde_json::from_slice::<Block>(&blob.data)?;
                                assert_eq!(block.transactions[0].0, data);
                                return Ok(());
                            },
                            _ => {
                                assert!(false, "Invalid blob type in response");
                            }
                        }
                    },
                    None => {
                        assert!(false, "No blob in response");
                    }
                }    
                
            },
            Err(e) => {
                let log_line = format!("Error: {}", e);
                log_lines.push(log_line);
            },
        }

    }

    assert!(false, "No block fou in 16 attempts, log: {:?}", log_lines);

    Ok(())

}