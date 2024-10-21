use crate::set::{
	Cold, ColdError, ColdGuard, ColdGuardError, Hot, HotError, HotGuard, HotGuardError,
	Recoverable, RecoveryError, TryAsMember,
};
use rocksdb::{Options, DB};

pub struct HotSet {
	/// The underlying RocksDB instance
	pub rocksdb: DB,
}

pub struct ColdSet {
	/// The underlying RocksDB instance
	pub rocksdb: DB,
}

/// A hot guard for RocksDB
pub struct RocksHotGuard;

impl<M> HotGuard<M> for RocksHotGuard
where
	M: TryAsMember + Send,
{
	async fn rollback(&self, _members: &[M]) -> Result<(), HotGuardError> {
		Ok(()) // Assume rollback is always successful for simplicity
	}

	async fn commit(&self, _members: &[M]) -> Result<(), HotGuardError> {
		Ok(()) // Assume commit is always successful for simplicity
	}
}

/// A Hot Set implementation using RocksDB
impl HotSet {
	pub fn new(path: &str) -> Self {
		let mut options = Options::default();
		options.create_if_missing(true);
		let rocksdb = DB::open(&options, path).expect("Failed to open RocksDB for HotSet");
		HotSet { rocksdb }
	}
}

impl Recoverable for HotSet {
	async fn recover(&self) -> Result<(), RecoveryError> {
		todo!()
	}
}

impl<M> Hot<M, RocksHotGuard> for HotSet
where
	M: TryAsMember + Send,
{
	async fn cardinality(&self) -> Result<u64, HotError> {
		// Count the number of entries in the database
		let count = self.rocksdb.iterator(rocksdb::IteratorMode::Start).count();
		Ok(count as u64)
	}

	async fn ttl(&self) -> Result<u64, HotError> {
		// Placeholder for TTL logic, RocksDB doesn't have native TTL.
		Ok(60) // Dummy TTL value
	}

	async fn prepare_insert(&self, members: &[M]) -> Result<RocksHotGuard, HotError> {
		// Prepare the insert by checking the keys
		for member in members {
			let converted_member = member.try_as_member().map_err(|_| HotError::Internal)?;
			self.rocksdb
				.put(converted_member.0, b"prepared")
				.map_err(|_| HotError::Internal)?;
		}
		Ok(RocksHotGuard)
	}

	async fn contains(&self, members: &[M]) -> Result<bool, HotError> {
		for member in members {
			let converted_member = member.try_as_member().map_err(|_| HotError::Internal)?;
			if let Err(_) = self.rocksdb.get(converted_member.0) {
				return Ok(false);
			}
		}
		Ok(true)
	}

	async fn maybe_contained(&self, members: &[M]) -> Result<bool, HotError> {
		// todo: this will be replaced by a bloom filter in a future PR
		self.contains(members).await
	}

	async fn gc(&self) -> Result<(), HotError> {
		todo!()
	}
}

/// A cold guard for RocksDB
pub struct RocksColdGuard;

impl<M> ColdGuard<M> for RocksColdGuard
where
	M: TryAsMember + Send,
{
	async fn rollback(&self, _members: &[M]) -> Result<(), ColdGuardError> {
		Ok(()) // Assume rollback is always successful for simplicity
	}

	async fn commit(&self, _members: &[M]) -> Result<(), ColdGuardError> {
		Ok(()) // Assume commit is always successful for simplicity
	}
}

/// A Cold Set implementation using RocksDB
impl ColdSet {
	pub fn new(path: &str) -> Self {
		let mut options = Options::default();
		options.create_if_missing(true);
		let rocksdb = DB::open(&options, path).expect("Failed to open RocksDB for ColdSet");
		ColdSet { rocksdb }
	}
}

impl Recoverable for ColdSet {
	async fn recover(&self) -> Result<(), RecoveryError> {
		todo!()
	}
}

impl<M> Cold<M, RocksColdGuard> for ColdSet
where
	M: TryAsMember + Send,
{
	async fn prepare_insert(&self, members: &[M]) -> Result<RocksColdGuard, ColdError> {
		// Prepare the insert in Cold set by writing keys as "prepared"
		for member in members {
			let converted_member = member.try_as_member().map_err(|_| ColdError::Internal)?;
			self.rocksdb
				.put(converted_member.0, b"prepared")
				.map_err(|_| ColdError::Internal)?;
		}
		Ok(RocksColdGuard)
	}

	async fn contains(&self, members: &[M]) -> Result<bool, ColdError> {
		for member in members {
			let converted_member = member.try_as_member().map_err(|_| ColdError::Internal)?;
			if let Err(_) = self.rocksdb.get(converted_member.0) {
				return Ok(false);
			}
		}
		Ok(true)
	}

	async fn maybe_contained(&self, members: &[M]) -> Result<bool, ColdError> {
		self.contains(members).await
	}
}

#[cfg(test)]
pub mod test {
	use super::*;
	use crate::set::test::TestMember;
	use crate::set::HotColdSet; // create the actual set against which we test with the HotColdSet struct.
	use tempfile::TempDir;

	#[tokio::test]
	async fn test_hot_cold_set_with_rocksdb() -> Result<(), anyhow::Error> {
		// Create temporary directories for testing RocksDB
		let hot_temp_dir = TempDir::new().expect("Failed to create temporary directory for HotSet");
		let cold_temp_dir =
			TempDir::new().expect("Failed to create temporary directory for ColdSet");

		let hot_path = hot_temp_dir.path().to_str().ok_or(anyhow::anyhow!(
			"Failed to convert HotSet temporary directory path to string."
		))?;
		let cold_path = cold_temp_dir.path().to_str().ok_or(anyhow::anyhow!(
			"Failed to convert ColdSet temporary directory path to string."
		))?;

		// Create a Hot and Cold set using RocksDB
		let hot_set = HotSet::new(hot_path);
		let cold_set = ColdSet::new(cold_path);

		// Create the HotColdSet
		let mut hot_cold_set = HotColdSet::new(hot_set, cold_set);

		// Define some members
		let members = vec![TestMember(1), TestMember(2)];

		// Try inserting into both sets
		let result = hot_cold_set.insert(members.clone()).await;

		// Ensure the insertion was successful
		assert!(result.is_ok(), "Insertion should succeed for reliable Hot and Cold sets.");

		// Check both sets contain the members
		let hot_contains = hot_cold_set.hot().contains(&members).await?;
		let cold_contains = hot_cold_set.cold().contains(&members).await?;

		assert!(hot_contains, "Hot set should contain the members.");
		assert!(cold_contains, "Cold set should contain the members.");

		Ok(())
	}
}
