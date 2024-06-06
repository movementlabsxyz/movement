use crate::*;
use anyhow::Ok;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_light_node_submits_blob_over_stream() -> Result<(), anyhow::Error> {
	let mut client = LightNodeServiceClient::connect("http://0.0.0.0:30730").await?;

	let blob_write = BlobWrite { data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9] };
	let request = StreamWriteBlobRequest { blob: Some(blob_write.clone()) };

	let (tx, rx) = tokio::sync::mpsc::channel(32);

	// Convert the receiver into a stream
	let stream = ReceiverStream::new(rx);

	let handle = client.stream_write_blob(stream).await?;

	tx.send(request.clone()).await?;

	let back = handle
		.into_inner()
		.next()
		.await
		.ok_or(anyhow::anyhow!("No response from server"))??;

	match back.blob {
		Some(blob) => match blob.blob_type.ok_or(anyhow::anyhow!("No blob type in response"))? {
			blob_response::BlobType::PassedThroughBlob(blob) => {
				assert_eq!(blob.data, request.blob.unwrap().data);
			}
			_ => {
				assert!(false, "Invalid blob type in response");
			}
		},
		None => {
			assert!(false, "No blob in response");
		}
	}

	Ok(())
}

#[tokio::test]
async fn test_submit_and_read() -> Result<(), anyhow::Error> {
	let mut client = LightNodeServiceClient::connect("http://0.0.0.0:30730").await?;

	let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
	let blob_write = BlobWrite { data: data.clone() };
	let request = BatchWriteRequest { blobs: vec![blob_write.clone()] };

	let write = client.batch_write(request).await?.into_inner();
	let first = write.blobs[0].clone();

	let blob_type = first.blob_type.ok_or(anyhow::anyhow!("No blob type in response"))?;
	let height = match blob_type {
		blob_response::BlobType::PassedThroughBlob(blob) => blob.height,
		_ => {
			anyhow::bail!("Invalid blob type in response");
		}
	};
	let read_request = ReadAtHeightRequest { height };

	let read = client.read_at_height(read_request).await?.into_inner();
	let first = read.blobs[0].clone();

	match first.blob_type.ok_or(anyhow::anyhow!("No blob type in response"))? {
		blob_response::BlobType::PassedThroughBlob(blob) => {
			assert_eq!(blob.data, data);
		}
		_ => {
			anyhow::bail!("Invalid blob type in response");
		}
	}

	Ok(())
}
