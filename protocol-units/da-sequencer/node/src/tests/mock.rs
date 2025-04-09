use std::collections::BTreeSet;
use std::future::Future;
use std::sync::{Arc, Mutex};

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use futures::StreamExt;

use movement_da_sequencer_client::{
	serialize_full_node_batch, DaSequencerClient, StreamReadBlockFromHeight,
};
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_signer::cryptography::ed25519::Signature as SigningSignature;
use movement_types::block::{Block, BlockMetadata, Id};
use movement_types::transaction::Transaction;

use crate::{
	batch::{DaBatch, FullNodeTxs},
	block::{BlockHeight, SequencerBlockDigest},
	celestia::blob::CelestiaBlobData,
	celestia::CelestiaHeight,
	DaSequencerError, DaSequencerExternalDa, DaSequencerStorage, SequencerBlock,
};

pub async fn mock_write_new_batch(
	client: &mut impl DaSequencerClient,
	signing_key: &SigningKey,
	verifying_key: VerifyingKey,
) {
	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);

	//sign batch
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	// Serialize full node batch into raw bytes
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	assert!(res.answer);
}

pub async fn mock_wait_and_get_next_block(
	block_stream: &mut StreamReadBlockFromHeight,
	expected_height: u64,
) {
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	loop {
		match tokio::time::timeout(std::time::Duration::from_secs(1), block_stream.next()).await {
			Ok(Some(Ok(block))) => {
				assert_eq!(
					block.height, expected_height,
					"stream block height 1, not the right height"
				);
				break;
			}
			_ => panic!("No block produced at height {expected_height}"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct StorageMockInternal {
	pub batches: Vec<DaBatch<FullNodeTxs>>,
	pub produced_blocks: Vec<SequencerBlock>,
	pub current_height: u64,
	pub parent_block_id: Id,
}

#[derive(Debug, Clone)]
pub struct StorageMock {
	pub inner: Arc<Mutex<StorageMockInternal>>,
}

impl StorageMock {
	pub fn new() -> Self {
		let inner = StorageMockInternal {
			batches: Vec::new(),
			current_height: 0,
			produced_blocks: vec![],
			parent_block_id: Id::genesis_block(),
		};
		StorageMock { inner: Arc::new(Mutex::new(inner)) }
	}
}

impl DaSequencerStorage for StorageMock {
	fn write_batch(
		&self,
		batch: DaBatch<FullNodeTxs>,
	) -> std::result::Result<(), DaSequencerError> {
		tracing::info!("Mock: Storage, call write_batch");
		let mut inner = self.inner.lock().unwrap();
		inner.batches.push(batch);
		Ok(())
	}

	fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		let inner = self.inner.lock().unwrap();
		Ok(inner.produced_blocks.iter().find(|b| b.height == height).cloned())
	}

	fn get_block_with_digest(
		&self,
		id: SequencerBlockDigest,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		let inner = self.inner.lock().unwrap();
		Ok(inner.produced_blocks.iter().find(|b| b.get_block_digest() == id).cloned())
	}

	fn produce_next_block(&self) -> Result<Option<SequencerBlock>, DaSequencerError> {
		let mut inner = self.inner.lock().unwrap();
		if inner.batches.len() == 0 {
			return Ok(None);
		}
		let tx_list: BTreeSet<Transaction> =
			inner.batches.drain(..).flat_map(|b| b.data.0).collect();
		let block = Block::new(BlockMetadata::default(), inner.parent_block_id, tx_list);
		inner.parent_block_id = block.id();
		inner.current_height += 1;
		let sequencer_block = SequencerBlock::try_new(BlockHeight(inner.current_height), block)?;
		inner.produced_blocks.push(sequencer_block.clone());
		tracing::info!("Mock Storage produce block at height:{}", inner.current_height);
		Ok(Some(sequencer_block))
	}

	fn get_celestia_height_for_block(
		&self,
		heigh: BlockHeight,
	) -> std::result::Result<Option<CelestiaHeight>, DaSequencerError> {
		todo!();
	}

	fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> std::result::Result<(), DaSequencerError> {
		todo!();
	}

	fn get_current_block_height(&self) -> BlockHeight {
		self.inner.lock().unwrap().current_height.into()
	}
}

#[derive(Debug, Clone)]
pub struct CelestiaMock {}

impl CelestiaMock {
	pub fn new() -> Self {
		CelestiaMock {}
	}
}

impl DaSequencerExternalDa for CelestiaMock {
	fn send_block(
		&self,
		block: SequencerBlockDigest,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}

	fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlobData>, DaSequencerError>> + Send {
		//TODO return dummy error for now.
		futures::future::ready(Err(DaSequencerError::DeserializationFailure))
	}

	fn bootstrap(
		&self,
		current_block_height: BlockHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}
}
