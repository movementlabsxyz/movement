use mempool_util::{MempoolBlockOperations, MempoolTransactionOperations};
pub use move_rocks::RocksdbMempool;
pub use movement_types::{Block, Id, Transaction};
pub use sequencing_util::Sequencer;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Memseq<T: MempoolBlockOperations + MempoolTransactionOperations> {
	mempool: T,
	// this value should not be changed after initialization
	block_size: u32,
	pub parent_block: Arc<RwLock<Id>>,
	// this value should not be changed after initialization
	building_time_ms: u64,
}

impl<T: MempoolBlockOperations + MempoolTransactionOperations> Memseq<T> {
	pub fn new(
		mempool: T,
		block_size: u32,
		parent_block: Arc<RwLock<Id>>,
		building_time_ms: u64,
	) -> Self {
		Self { mempool, block_size, parent_block, building_time_ms }
	}

	pub fn with_block_size(mut self, block_size: u32) -> Self {
		self.block_size = block_size;
		self
	}

	pub fn with_building_time_ms(mut self, building_time_ms: u64) -> Self {
		self.building_time_ms = building_time_ms;
		self
	}

	pub fn building_time_ms(&self) -> u64 {
		self.building_time_ms
	}

}

impl Memseq<RocksdbMempool> {
	pub fn try_move_rocks(path: PathBuf) -> Result<Self, anyhow::Error> {
		let mempool = RocksdbMempool::try_new(
			path.to_str().ok_or(anyhow::anyhow!("PathBuf to str failed"))?,
		)?;
		let parent_block = Arc::new(RwLock::new(Id::default()));
		Ok(Self::new(mempool, 512, parent_block, 500))
	}

	pub fn try_from_env_toml_file() -> Result<Self, anyhow::Error> {
		unimplemented!("try_from_env_toml_file")
	}
}

impl<T: MempoolBlockOperations + MempoolTransactionOperations> Sequencer for Memseq<T> {

	async fn publish_many(&self, transactions: Vec<Transaction>) -> Result<(), anyhow::Error> {
		self.mempool.add_transactions(transactions).await?;
		Ok(())
	}

	async fn publish(&self, transaction: Transaction) -> Result<(), anyhow::Error> {
		self.mempool.add_transaction(transaction).await?;
		Ok(())
	}

	async fn wait_for_next_block(&self) -> Result<Option<Block>, anyhow::Error> {
		let mut transactions = Vec::with_capacity(self.block_size as usize);

		let mut now = std::time::Instant::now();

		loop {
			let current_block_size = transactions.len() as u32;
			if current_block_size >= self.block_size {
				break;
			}

			let remaining = self.block_size - current_block_size;
			let mut transactions_to_add = self.mempool.pop_transactions(remaining as usize).await?;
			transactions.append(&mut transactions_to_add);

			// sleep to yield to other tasks and wait for more transactions
			tokio::task::yield_now().await;

			if now.elapsed().as_millis() as u64 > self.building_time_ms {
				break;
			}
		}

		if transactions.is_empty() {
			Ok(None)
		} else {

			let new_block = {
				let parent_block = self.parent_block.read().await.clone();
				Block::new(Default::default(), parent_block.to_vec(), transactions)
			};
			
			// update the parent block 
			{
				let mut parent_block = self.parent_block.write().await;
				*parent_block = new_block.id();
			}

			Ok(Some(new_block))
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use futures::stream::FuturesUnordered;
	use futures::StreamExt;
	use mempool_util::MempoolTransaction;
	use tempfile::tempdir;

	#[tokio::test]
	async fn test_wait_for_next_block_building_time_expires() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Memseq::try_move_rocks(path)?.with_block_size(10).with_building_time_ms(500);

		// Add some transactions
		for i in 0..5 {
			let transaction = Transaction::new(vec![i as u8], 0);
			memseq.publish(transaction).await?;
		}

		// Wait for the block to be built, not enough transactions as such
		// the building time should expire
		let block = memseq.wait_for_next_block().await?;
		assert!(block.is_some());

		let block = block.ok_or(anyhow::anyhow!("Block not found"))?;
		assert_eq!(block.transactions.len(), 5);

		Ok(())
	}

	#[tokio::test]
	async fn test_publish_error_propagation() -> Result<(), anyhow::Error> {
		let mempool = MockMempool;
		let parent_block = Arc::new(RwLock::new(Id::default()));
		let memseq = Memseq::new(mempool, 10, parent_block, 1000);

		let transaction = Transaction::new(vec![1, 2, 3], 0);
		let result = memseq.publish(transaction).await;
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "Mock add_transaction");

		let result = memseq.wait_for_next_block().await;
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "Mock pop_transaction");

		Ok(())
	}

	#[tokio::test]
	async fn test_concurrent_access_spawn() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Arc::new(Memseq::try_move_rocks(path)?);

		let mut handles = vec![];

		for i in 0..100 {
			let memseq_clone = Arc::clone(&memseq);
			let handle = tokio::spawn(async move {
				let transaction = Transaction::new(vec![i as u8], 0);
				memseq_clone.publish(transaction).await.unwrap();
			});
			handles.push(handle);
		}

		for handle in handles {
			handle.await.expect("Task failed");
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_concurrent_access_futures() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Arc::new(Memseq::try_move_rocks(path)?);

		let futures = FuturesUnordered::new();

		for i in 0..10 {
			let memseq_clone = Arc::clone(&memseq);
			let handle = async move {
				for n in 0..10 {
					let transaction = Transaction::new(vec![i * 10 + n as u8], 0);
					memseq_clone.publish(transaction).await?;
				}
				Ok::<_, anyhow::Error>(())
			};
			futures.push(handle);
		}

		let all_executed_correctly = futures.all(|result| async move { result.is_ok() }).await;
		assert!(all_executed_correctly);

		Ok(())
	}

	#[tokio::test]
	async fn test_try_move_rocks() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Memseq::try_move_rocks(path.clone())?;

		assert_eq!(memseq.block_size, 1024);
		assert_eq!(memseq.building_time_ms, 500);

		// Test invalid path
		let invalid_path = PathBuf::from("");
		let result = Memseq::try_move_rocks(invalid_path);
		assert!(result.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_memseq_initialization() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();

		let mem_pool = RocksdbMempool::try_new(
			path.to_str().ok_or(anyhow::anyhow!("PathBuf to str failed"))?,
		)?;
		let block_size = 50;
		let building_time_ms = 2000;
		let parent_block = Arc::new(RwLock::new(Id::default()));

		let memseq = Memseq::new(mem_pool, block_size, Arc::clone(&parent_block), building_time_ms);

		assert_eq!(memseq.block_size, block_size);
		assert_eq!(memseq.building_time_ms, building_time_ms);
		assert_eq!(*memseq.parent_block.read().await, *parent_block.read().await);

		Ok(())
	}

	#[tokio::test]
	async fn test_memseq_with_methods() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();

		let mem_pool = RocksdbMempool::try_new(
			path.to_str().ok_or(anyhow::anyhow!("PathBuf to str failed"))?,
		)?;
		let block_size = 50;
		let building_time_ms = 2000;
		let parent_block = Arc::new(RwLock::new(Id::default()));

		let memseq = Memseq::new(mem_pool, block_size, Arc::clone(&parent_block), building_time_ms);

		// Test with_block_size
		let new_block_size = 100;
		let memseq = memseq.with_block_size(new_block_size);
		assert_eq!(memseq.block_size, new_block_size);

		// Test with_building_time_ms
		let new_building_time_ms = 5000;
		let memseq = memseq.with_building_time_ms(new_building_time_ms);
		assert_eq!(memseq.building_time_ms, new_building_time_ms);

		Ok(())
	}

	#[tokio::test]
	async fn test_wait_for_next_block_no_transactions() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Memseq::try_move_rocks(path)?.with_block_size(10).with_building_time_ms(500);

		let block = memseq.wait_for_next_block().await?;
		assert!(block.is_none());

		Ok(())
	}

	#[tokio::test]
	async fn test_memseq() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let memseq = Memseq::try_move_rocks(path)?;

		let transaction : Transaction = Transaction::new(vec![1, 2, 3], 0);
		memseq.publish(transaction.clone()).await?;

		let block = memseq.wait_for_next_block().await?;

		assert_eq!(block.ok_or(anyhow::anyhow!("Block not found"))?.transactions[0], transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_respects_size() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let block_size = 100;
		let memseq = Memseq::try_move_rocks(path)?.with_block_size(block_size);

		let mut transactions = Vec::new();
		for i in 0..block_size * 2 {
			let transaction : Transaction = Transaction::new( vec![i as u8], 0);
			memseq.publish(transaction.clone()).await?;
			transactions.push(transaction);
		}

		let block = memseq.wait_for_next_block().await?;

		assert!(block.is_some());

		let block = block.ok_or(anyhow::anyhow!("Block not found"))?;

		assert_eq!(block.transactions.len(), block_size as usize);

		let second_block = memseq.wait_for_next_block().await?;

		assert!(second_block.is_some());

		let second_block = second_block.ok_or(anyhow::anyhow!("Second block not found"))?;

		assert_eq!(second_block.transactions.len(), block_size as usize);

		Ok(())
	}

	#[tokio::test]
	async fn test_wait_next_block_respects_time() -> Result<(), anyhow::Error> {
		let dir = tempdir()?;
		let path = dir.path().to_path_buf();
		let block_size = 100;
		let memseq = Memseq::try_move_rocks(path)?
			.with_block_size(block_size)
			.with_building_time_ms(500);

		let building_memseq = Arc::new(memseq);
		let waiting_memseq = Arc::clone(&building_memseq);

		let building_task = async move {
			let memseq = building_memseq;

			// add half of the transactions
			for i in 0..block_size / 2 {
				let transaction : Transaction = Transaction::new(vec![i as u8], 0);
				memseq.publish(transaction.clone()).await?;
			}

			tokio::time::sleep(std::time::Duration::from_millis(600)).await;

			// add the rest of the transactions
			for i in block_size / 2..block_size - 2 {
				let transaction : Transaction = Transaction::new(vec![i as u8], 0);
				memseq.publish(transaction.clone()).await?;
			}

			Ok::<_, anyhow::Error>(())
		};

		let waiting_task = async move {
			let memseq = waiting_memseq;

			// first block
			let block = memseq.wait_for_next_block().await?;
			assert!(block.is_some());
			let block = block.ok_or(anyhow::anyhow!("Block not found"))?;
			assert_eq!(block.transactions.len(), (block_size / 2) as usize);

			tokio::time::sleep(std::time::Duration::from_millis(200)).await;

			// second block
			let block = memseq.wait_for_next_block().await?;
			assert!(block.is_some());
			let block = block.ok_or(anyhow::anyhow!("Block not found"))?;
			assert_eq!(block.transactions.len(), ((block_size / 2) - 2) as usize);

			Ok::<_, anyhow::Error>(())
		};

		tokio::try_join!(building_task, waiting_task)?;

		Ok(())
	}

	/// Mock Mempool
	struct MockMempool;
	impl MempoolTransactionOperations for MockMempool {
		async fn has_mempool_transaction(
			&self,
			_transaction_id: Id,
		) -> Result<bool, anyhow::Error> {
			Err(anyhow::anyhow!("Mock has_mempool_transaction"))
		}

		async fn add_mempool_transactions(
			&self,
			_transactions: Vec<MempoolTransaction>,
		) -> Result<(), anyhow::Error> {
			Err(anyhow::anyhow!("Mock add_mempool_transactions"))
		}

		async fn add_mempool_transaction(
			&self,
			_tx: MempoolTransaction,
		) -> Result<(), anyhow::Error> {
			Err(anyhow::anyhow!("Mock add_mempool_transaction"))
		}

		async fn remove_mempool_transaction(
			&self,
			_transaction_id: Id,
		) -> Result<(), anyhow::Error> {
			Err(anyhow::anyhow!("Mock remove_mempool_transaction"))
		}

		async fn pop_mempool_transaction(
			&self,
		) -> Result<Option<MempoolTransaction>, anyhow::Error> {
			Err(anyhow::anyhow!("Mock pop_mempool_transaction"))
		}

		async fn get_mempool_transaction(
			&self,
			_transaction_id: Id,
		) -> Result<Option<MempoolTransaction>, anyhow::Error> {
			Err(anyhow::anyhow!("Mock get_mempool_transaction"))
		}

		async fn add_transaction(&self, _transaction: Transaction) -> Result<(), anyhow::Error> {
			Err(anyhow::anyhow!("Mock add_transaction"))
		}

		async fn pop_transaction(&self) -> Result<Option<Transaction>, anyhow::Error> {
			Err(anyhow::anyhow!("Mock pop_transaction"))
		}
	}

	impl MempoolBlockOperations for MockMempool {
		async fn has_block(&self, _block_id: Id) -> Result<bool, anyhow::Error> {
			todo!()
		}

		async fn add_block(&self, _block: Block) -> Result<(), anyhow::Error> {
			todo!()
		}

		async fn remove_block(&self, _block_id: Id) -> Result<(), anyhow::Error> {
			todo!()
		}

		async fn get_block(&self, _block_id: Id) -> Result<Option<Block>, anyhow::Error> {
			todo!()
		}
	}
}
