use crate::set::{Cold, ColdGuard, Hot, HotColdSet, HotGuard, Member, TryAsMember};
use std::collections::BTreeSet;
use thiserror::Error;

#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "rest")]
pub mod rest;

#[derive(Debug, Error)]
pub enum CheckpointError {
	#[error("Failed to enumerate checkpoints: {0}")]
	EnumerateCheckpoints(String),
	#[error("Failed to get inner data: {0}")]
	GetInnerData(String),
}

pub trait Checkpoint: Sized + PartialEq + Eq + PartialOrd + Ord + TryAsMember {
	/// The inner data type around which the checkpoint is built.
	type Data;

	/// Enumerates all possible checkpoints for given inner data.
	fn enumerate_checkpoints(inner: &Self::Data) -> Result<BTreeSet<Self>, CheckpointError>;

	/// Enumerates all possible checkpoints for given data as members
	fn enumerate_checkpoints_as_members(
		inner: &Self::Data,
	) -> Result<Vec<Member>, CheckpointError> {
		Self::enumerate_checkpoints(inner)?
			.into_iter()
			.map(|checkpoint| {
				checkpoint.try_as_member().map_err(|_| {
					CheckpointError::EnumerateCheckpoints(
						"Failed to convert checkpoint to member".to_string(),
					)
				})
			})
			.collect()
	}

	/// Tries to return the inner data of the checkpoint.
	fn try_inner(&self) -> Result<&Self::Data, CheckpointError>;

	/// Returns the satisfying checkpoints for the given inner data.
	fn satisfying_checkpoints(&self) -> Result<BTreeSet<Self>, CheckpointError> {
		// Get all of the checkpoints for the inner data.
		let inner = self.try_inner()?;
		let all_checkpoints = Self::enumerate_checkpoints(inner)?;

		// Get all of the checkpoints that come before or include the current checkpoint.
		let satisfying_checkpoints = all_checkpoints
			.into_iter()
			.take_while(|checkpoint| checkpoint <= self)
			.collect();

		Ok(satisfying_checkpoints)
	}
}

pub struct CheckpointHotColdSet<M, HG, CG, H, C>
where
	M: Checkpoint,
	HG: HotGuard<M>,
	CG: ColdGuard<M>,
	H: Hot<M, HG>,
	C: Cold<M, CG>,
{
	hot_cold_set: HotColdSet<M, HG, CG, H, C>,
}

#[derive(Debug, Error)]
pub enum CheckpointHotColdSetError {
	#[error("Failed to enumerate checkpoints: {0}")]
	InternalError(String),
}

impl<M, HG, CG, H, C> CheckpointHotColdSet<M, HG, CG, H, C>
where
	M: Checkpoint,
	HG: HotGuard<M>,
	CG: ColdGuard<M>,
	H: Hot<M, HG>,
	C: Cold<M, CG>,
{
	pub fn new(hot: H, cold: C) -> Self {
		CheckpointHotColdSet { hot_cold_set: HotColdSet::new(hot, cold) }
	}

	/// Checks whether the hot set contains the given checkpoints.
	pub async fn hot_reached(&self, checkpoints: &[M]) -> Result<bool, CheckpointHotColdSetError> {
		self.hot_cold_set.hot().contains(checkpoints).await.map_err(|e| {
			CheckpointHotColdSetError::InternalError(format!(
				"Failed to check if hot set contains checkpoints: {:?}",
				e
			))
		})
	}

	/// Checks probabilistically whether the hot set contains the given checkpoints.
	pub async fn hot_maybe_reached(
		&self,
		checkpoints: &[M],
	) -> Result<bool, CheckpointHotColdSetError> {
		self.hot_cold_set.hot().maybe_contained(checkpoints).await.map_err(|e| {
			CheckpointHotColdSetError::InternalError(format!(
				"Failed to check if hot set maybe contains checkpoints: {:?}",
				e
			))
		})
	}

	/// Checks whether the cold set contains the given checkpoints.
	pub async fn cold_reached(&self, checkpoints: &[M]) -> Result<bool, CheckpointHotColdSetError> {
		self.hot_cold_set.cold().contains(checkpoints).await.map_err(|e| {
			CheckpointHotColdSetError::InternalError(format!(
				"Failed to check if cold set contains checkpoints: {:?}",
				e
			))
		})
	}

	/// Checks whether all of the checkpoints were satisfied in the hot set.
	pub async fn hot_satisfied(
		&self,
		checkpoints: &[M],
	) -> Result<bool, CheckpointHotColdSetError> {
		let all_satisfying_checkpoints = checkpoints
			.iter()
			.map(|checkpoint| checkpoint.satisfying_checkpoints())
			.collect::<Result<Vec<BTreeSet<M>>, CheckpointError>>()
			.map_err(|e| {
				CheckpointHotColdSetError::InternalError(format!(
					"Failed to get satisfying checkpoints: {:?}",
					e
				))
			})?
			.into_iter()
			.flatten()
			.collect::<BTreeSet<M>>();

		// Convert the BTreeSet<M> to a Vec<M> and then to a slice
		let all_satisfying_checkpoints_vec: Vec<M> =
			all_satisfying_checkpoints.into_iter().collect();

		self.hot_reached(&all_satisfying_checkpoints_vec).await
	}

	/// Checks whether all of checkpoints were maybe satisfied in the hot set.
	pub async fn hot_maybe_satisfied(
		&self,
		checkpoints: &[M],
	) -> Result<bool, CheckpointHotColdSetError> {
		let all_satisfying_checkpoints = checkpoints
			.iter()
			.map(|checkpoint| checkpoint.satisfying_checkpoints())
			.collect::<Result<Vec<BTreeSet<M>>, CheckpointError>>()
			.map_err(|e| {
				CheckpointHotColdSetError::InternalError(format!(
					"Failed to get satisfying checkpoints: {:?}",
					e
				))
			})?
			.into_iter()
			.flatten()
			.collect::<BTreeSet<M>>();

		// Convert the BTreeSet<M> to a Vec<M> and then to a slice
		let all_satisfying_checkpoints_vec: Vec<M> =
			all_satisfying_checkpoints.into_iter().collect();

		self.hot_maybe_reached(&all_satisfying_checkpoints_vec).await
	}

	/// Checks whether all of the checkpoints were satisfied in the cold set.
	pub async fn cold_satisfied(
		&self,
		checkpoints: &[M],
	) -> Result<bool, CheckpointHotColdSetError> {
		let all_satisfying_checkpoints = checkpoints
			.iter()
			.map(|checkpoint| checkpoint.satisfying_checkpoints())
			.collect::<Result<Vec<BTreeSet<M>>, CheckpointError>>()
			.map_err(|e| {
				CheckpointHotColdSetError::InternalError(format!(
					"Failed to get satisfying checkpoints: {:?}",
					e
				))
			})?
			.into_iter()
			.flatten()
			.collect::<BTreeSet<M>>();

		// Convert the BTreeSet<M> to a Vec<M> and then to a slice
		let all_satisfying_checkpoints_vec: Vec<M> =
			all_satisfying_checkpoints.into_iter().collect();

		self.cold_reached(&all_satisfying_checkpoints_vec).await
	}

	/// Borrows the hot cold set
	pub fn hot_cold_set(&self) -> &HotColdSet<M, HG, CG, H, C> {
		&self.hot_cold_set
	}

	/// Borrows the hot cold set mutably
	pub fn hot_cold_set_mut(&mut self) -> &mut HotColdSet<M, HG, CG, H, C> {
		&mut self.hot_cold_set
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::set::{
		test::{ReliableCold, ReliableHot},
		Member, MemberError,
	};
	use serde::{Deserialize, Serialize};
	use std::collections::BTreeSet;
	use std::collections::HashSet;
	use std::sync::Arc;
	use tokio::sync::RwLock;

	/// A < B < C
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
	pub enum TestCheckpoint {
		A(u8),
		B(u8),
		C(u8),
	}

	impl TryAsMember for TestCheckpoint {
		fn try_as_member(&self) -> Result<Member, MemberError> {
			// serialize the checkpoint
			let serialized = serde_json::to_vec(self).map_err(|_| MemberError::ConversionFailed)?;
			Ok(Member(serialized))
		}
	}

	impl Checkpoint for TestCheckpoint {
		type Data = u8;

		fn enumerate_checkpoints(inner: &Self::Data) -> Result<BTreeSet<Self>, CheckpointError> {
			let mut checkpoints = BTreeSet::new();
			checkpoints.insert(TestCheckpoint::A(*inner));
			checkpoints.insert(TestCheckpoint::B(*inner));
			checkpoints.insert(TestCheckpoint::C(*inner));
			Ok(checkpoints)
		}

		fn try_inner(&self) -> Result<&Self::Data, CheckpointError> {
			match self {
				TestCheckpoint::A(inner) => Ok(inner),
				TestCheckpoint::B(inner) => Ok(inner),
				TestCheckpoint::C(inner) => Ok(inner),
			}
		}
	}

	#[tokio::test]
	pub async fn test_checkpoint_set_simple() -> Result<(), anyhow::Error> {
		// Create a reliable Hot set
		let hot =
			ReliableHot { cardinality: 0, ttl: 60, set: Arc::new(RwLock::new(HashSet::new())) };

		// Create a reliable Cold set
		let cold = ReliableCold { set: Arc::new(RwLock::new(HashSet::new())) };

		// Create a CheckpointHotColdSet
		let mut checkpoint_hot_cold_set: CheckpointHotColdSet<TestCheckpoint, _, _, _, _> =
			CheckpointHotColdSet::new(hot, cold);

		// Create a checkpoint
		let checkpoint = TestCheckpoint::A(1);

		// Check if the hot set contains the checkpoint
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[checkpoint]).await?;
		assert_eq!(hot_reached, false);

		// Check if the hot set maybe contains the checkpoint
		let hot_maybe_reached = checkpoint_hot_cold_set.hot_maybe_reached(&[checkpoint]).await?;
		assert_eq!(hot_maybe_reached, false);

		// Check if the cold set contains the checkpoint
		let cold_reached = checkpoint_hot_cold_set.cold_reached(&[checkpoint]).await?;
		assert_eq!(cold_reached, false);

		// Check if the hot set contains the satisfying checkpoints
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[checkpoint]).await?;
		assert_eq!(hot_satisfied, false);

		// Check if the hot set maybe contains the satisfying checkpoints
		let hot_maybe_satisfied =
			checkpoint_hot_cold_set.hot_maybe_satisfied(&[checkpoint]).await?;
		assert_eq!(hot_maybe_satisfied, false);

		// Check if the cold set contains the satisfying checkpoints
		let cold_satisfied = checkpoint_hot_cold_set.cold_satisfied(&[checkpoint]).await?;
		assert_eq!(cold_satisfied, false);

		// Add the checkpoint to the hot cold set
		checkpoint_hot_cold_set.hot_cold_set_mut().insert(vec![checkpoint]).await?;

		// Check if the hot set contains the checkpoint
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[checkpoint]).await?;
		assert_eq!(hot_reached, true);

		// Check if the hot set maybe contains the checkpoint
		let hot_maybe_reached = checkpoint_hot_cold_set.hot_maybe_reached(&[checkpoint]).await?;
		assert_eq!(hot_maybe_reached, true);

		// Check if the cold set contains the checkpoint
		let cold_reached = checkpoint_hot_cold_set.cold_reached(&[checkpoint]).await?;
		assert_eq!(cold_reached, false);

		// Check if the hot set contains the satisfying checkpoints
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[checkpoint]).await?;
		assert_eq!(hot_satisfied, true);

		// Check if the hot set maybe contains the satisfying checkpoints
		let hot_maybe_satisfied =
			checkpoint_hot_cold_set.hot_maybe_satisfied(&[checkpoint]).await?;
		assert_eq!(hot_maybe_satisfied, true);

		// Check if the cold set contains the satisfying checkpoints
		let cold_satisfied = checkpoint_hot_cold_set.cold_satisfied(&[checkpoint]).await?;
		assert_eq!(cold_satisfied, false);

		Ok(())
	}

	#[tokio::test]
	pub async fn test_checkpoint_set_satisfied() -> Result<(), anyhow::Error> {
		// Create a reliable Hot set
		let hot =
			ReliableHot { cardinality: 0, ttl: 60, set: Arc::new(RwLock::new(HashSet::new())) };

		// Create a reliable Cold set
		let cold = ReliableCold { set: Arc::new(RwLock::new(HashSet::new())) };

		// Create a CheckpointHotColdSet
		let mut checkpoint_hot_cold_set: CheckpointHotColdSet<TestCheckpoint, _, _, _, _> =
			CheckpointHotColdSet::new(hot, cold);

		// Insert C(1) into the set
		checkpoint_hot_cold_set
			.hot_cold_set_mut()
			.insert(vec![TestCheckpoint::C(1)])
			.await?;

		// C(1) should be reached in the hot set
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[TestCheckpoint::C(1)]).await?;
		assert_eq!(hot_reached, true);

		// C(1) should not be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::C(1)]).await?;
		assert_eq!(hot_satisfied, false);

		// Insert B(1) into the set
		checkpoint_hot_cold_set
			.hot_cold_set_mut()
			.insert(vec![TestCheckpoint::B(1)])
			.await?;

		// B(1) should be reached in the hot set
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[TestCheckpoint::B(1)]).await?;
		assert_eq!(hot_reached, true);

		// B(1) should not be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::B(1)]).await?;
		assert_eq!(hot_satisfied, false);

		// C(1) should not be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::C(1)]).await?;
		assert_eq!(hot_satisfied, false);

		// Insert A(2) into the set
		checkpoint_hot_cold_set
			.hot_cold_set_mut()
			.insert(vec![TestCheckpoint::A(2)])
			.await?;

		// A(2) should be reached in the hot set
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[TestCheckpoint::A(2)]).await?;
		assert_eq!(hot_reached, true);

		// A(2) should be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::A(2)]).await?;
		assert_eq!(hot_satisfied, true);

		// B(1) should not be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::B(1)]).await?;
		assert_eq!(hot_satisfied, false);

		// C(1) should not be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::C(1)]).await?;
		assert_eq!(hot_satisfied, false);

		// Insert A(1) into the set
		checkpoint_hot_cold_set
			.hot_cold_set_mut()
			.insert(vec![TestCheckpoint::A(1)])
			.await?;

		// A(1) should be reached in the hot set
		let hot_reached = checkpoint_hot_cold_set.hot_reached(&[TestCheckpoint::A(1)]).await?;
		assert_eq!(hot_reached, true);

		// A(1) should be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::A(1)]).await?;
		assert_eq!(hot_satisfied, true);

		// B(1) should be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::B(1)]).await?;
		assert_eq!(hot_satisfied, true);

		// C(1) should be satisfied in the hot set
		let hot_satisfied = checkpoint_hot_cold_set.hot_satisfied(&[TestCheckpoint::C(1)]).await?;

		Ok(())
	}
}
