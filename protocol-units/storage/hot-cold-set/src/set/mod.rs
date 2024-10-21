// ! The [HotColdSet] struct describes a synchronization primitive used to ensure consistent inclusion of members between two sets, known as Hot and Cold sets.
// ! The synchronization protocol is similar to a 3-way handshake, consisting of multiple exchanges between Hot and Cold sets to commit the inclusion of a member.
// !
// ! The Hot set is considered hot for two reasons:
// ! 1. It is written to first and read from first by the application.
// ! 2. The Hot set can be garbage collected.
// ! The Cold set is considered cold for two reasons:
// ! 1. It is written to second and serves as a backup to the Hot set.
// ! 2. The Cold set is append-only and is never garbage collected by the application.
// !
// ! Originally designed for event sets, this protocol can be extended to other contexts, such as synchronization of transactions.
// ! Implementers should be cautious of failure points, as frequent commit failures will flag the system as inconsistent and induce recovery attempts.
use std::marker::PhantomData;
use thiserror::Error;

/// A member of the set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Member(pub Vec<u8>);

/// An error thrown when a member cannot be converted into a common representation.
#[derive(Debug, Error)]
pub enum MemberError {
	#[error("failed to convert member into a common representation")]
	ConversionFailed,
}

/// A member of the set should implement TryAsMember to convert itself into a common representation.
pub trait TryAsMember {
	fn try_as_member(&self) -> Result<Member, MemberError>;
}

#[derive(Debug, Error)]
pub enum RecoveryError {
	#[error("failed to recover the set")]
	Irrecoverable,
}

/// Hot and Cold sets should be recoverable
pub trait Recoverable {
	async fn recover(&self) -> Result<(), RecoveryError>;
}

/// Error types for the Hot guard.
#[derive(Debug, Error)]
pub enum HotGuardError {
	#[error("hot guard failed to rollback the insertion")]
	FailedToRollback,
	#[error("hot guard failed to commit the insertion")]
	FailedToCommit,
}

/// A Hot guard responsible for rollback or commit operations.
pub trait HotGuard<M> where M: TryAsMember {
	/// Rollback the prepared insertion in the Hot set.
	async fn rollback(&self, members: &[M]) -> Result<(), HotGuardError>;

	/// Commit the prepared insertion in the Hot set.
	async fn commit(&self, members: &[M]) -> Result<(), HotGuardError>;
}

#[derive(Debug, Error)]
pub enum HotError {
	#[error("internal error")]
	Internal,
}

/// The Hot portion of the set, optimized for fast access and size.
/// The Hot set is garbage collected and typically uses structures like Bloom filters to probabilistically check membership.
/// The Hot set supports asynchronous operations and should implement this trait with a type-specific guard.
pub trait Hot<M, G> : Recoverable
where
	M: TryAsMember,
	G: HotGuard<M>,
{
	/// Get the intended cardinality of the set.
	async fn cardinality(&self) -> Result<u64, HotError>;

	/// Get the time-to-live (TTL) of members in the set.
	async fn ttl(&self) -> Result<u64, HotError>;

	/// Prepare to insert a collection of members into the Hot set.
    async fn prepare_insert(&self, members: &[M]) -> Result<G, HotError>;

    /// Check if a collection of members is in the Hot set.
    async fn contains(&self, members: &[M]) -> Result<bool, HotError>;

    /// Check if the Hot set likely contained a collection of members.
    async fn maybe_contained(&self, members: &[M]) -> Result<bool, HotError>;

	/// Garbage collect the Hot set.
	async fn gc(&self) -> Result<(), HotError>;
}

#[derive(Debug, Error)]
pub enum ColdGuardError {
	#[error("cold guard failed to commit the insertion")]
	FailedToCommit,
}

/// A Cold guard responsible for commit and rollback operations in the Cold set.
pub trait ColdGuard<M> where M: TryAsMember {
	/// Rollback the prepared insertion in the Cold set.
	async fn rollback(&self, members: &[M]) -> Result<(), ColdGuardError>;

	/// Commit the prepared insertion in the Cold set.
	async fn commit(&self, members: &[M]) -> Result<(), ColdGuardError>;
}

#[derive(Debug, Error)]
pub enum ColdError {
	#[error("internal error")]
	Internal
}

/// The Cold portion of the set is append-only and intended for long-term storage.
/// The Cold set serves as a backup and is never garbage collected by the application.
pub trait Cold<M, G> : Recoverable
where
	M: TryAsMember,
	G: ColdGuard<M>,
{
	/// Prepare to insert a collection of members into the Hot set.
    async fn prepare_insert(&self, members: &[M]) -> Result<G, ColdError>;

    /// Check if a collection of members is in the Hot set.
    async fn contains(&self, members: &[M]) -> Result<bool, ColdError>;

    /// Check if the Hot set likely contained a collection of members.
    async fn maybe_contained(&self, members: &[M]) -> Result<bool, ColdError>;

}

/// Describes a partial insertion state in which the Hot or both Hot and Cold sets were partially committed.
#[derive(Debug)]
pub enum InsertionPartial {
	Hot,
	Both,
}

/// Error types for the Hot-Cold set operations.
#[derive(Debug, Error)]
pub enum HotColdError<M>
where
	M: TryAsMember,
{
	#[error("hot-cold set is inconsistent (partially committed)")]
	Inconsistent(InsertionPartial, Vec<M>),
	#[error("hot-cold set is already inconsistent")]
	Irrecoverable,
	#[error("failed to insert member")]
	FailedToInsert,
}

/// The `HotColdSet` struct ensures synchronized inclusion of members between the Hot and Cold sets.
pub struct HotColdSet<M, HG, CG, H, C>
where
	M: TryAsMember,
	HG: HotGuard<M>,
	CG: ColdGuard<M>,
	H: Hot<M, HG>,
	C: Cold<M, CG>,
{
	_member_marker: PhantomData<M>,
	_hot_guard_marker: PhantomData<HG>,
	_cold_guard_marker: PhantomData<CG>,
	hot: H,
	cold: C,
	is_consistent: bool,
}

impl<M, HG, CG, H, C> HotColdSet<M, HG, CG, H, C>
where
	M: TryAsMember,
	HG: HotGuard<M>,
	CG: ColdGuard<M>,
	H: Hot<M, HG>,
	C: Cold<M, CG>,
{

    /// Create a new Hot-Cold Set with the given Hot and Cold sets.
    pub fn new(hot: H, cold: C) -> Self {
        HotColdSet {
            _member_marker: PhantomData,
            _hot_guard_marker: PhantomData,
            _cold_guard_marker: PhantomData,
            hot,
            cold,
			is_consistent: true,
        }
    }

	/// Get the Hot set.
	pub fn hot(&self) -> &H {
		&self.hot
	}

	/// Get the Cold set.
	pub fn cold(&self) -> &C {
		&self.cold
	}

	/// Insert a member into both the Hot and Cold sets, ensuring consistency.
	pub(crate) async fn insert_raw(&self, members: Vec<M>) -> Result<(), HotColdError<M>> {
		// SYN: Prepare to insert the member into the Hot set.
		let hot_guard = self
			.hot()
			.prepare_insert(&members)
			.await
			.map_err(|_| HotColdError::FailedToInsert)?;

		// ACK: Prepare to insert the member into the Cold set.
		match self.cold().prepare_insert(&members).await {
			Ok(cold_guard) => {
				// SYN-ACK: Commit the Hot set.
				match hot_guard.commit(&members).await {
					Ok(_) => {
						// Commit the Cold set.

						cold_guard.commit(&members).await.map_err(|_| {
							// If this fails, then...
							// (a) the hot set is considered partially committed because it was prepared and committed while the cold set was only prepared, i.e., its state should actually be rolled back.
							// (b) the cold set is considered partially committed because it was prepared but not committed and not successfully rolled back.
							HotColdError::Inconsistent(InsertionPartial::Both, members)
						})?;
						Ok(())
					}
					Err(_) => {
						// If Hot set commit fails, attempt to rollback the Cold set.
						match cold_guard.rollback(&members).await {
							Ok(_) => {
								// If the rollback succeeded, then the hot set should still be reported as partially committed.
								Err(HotColdError::Inconsistent(InsertionPartial::Hot, members))
							}
							Err(_) => {
								// If this fails, then...
								// (a) the hot set is considered partially committed because it was prepared but never committed.
								// (b) the cold set is also considered partially committed because it was prepared but not committed and not successfully rolled back.
								Err(HotColdError::Inconsistent(InsertionPartial::Both, members))
							}
						}
					}
				}
			}
			Err(_) => {
				// If Cold set insertion fails, rollback the Hot set.
				hot_guard
					.rollback(&members)
					.await
					.map_err(|_| 
				        // If this fails, then the hot set is considered partially committed because it was prepared but never committed and not successfully rolled back.
                        // Meanwhile, the cold set was never successfully prepared, so it is not considered partially committed.
                        HotColdError::Inconsistent(InsertionPartial::Hot, members)
                    )?;
                // If we did rollback successfully, then this is just a failed insert.
				Err(HotColdError::FailedToInsert)
			}
		}
	}

	/// Insert members into both the Hot and Cold sets, sets consistency, prevents attempting to insert to an inconsistent set.
	pub async fn insert(&mut self, members: Vec<M>) -> Result<(), HotColdError<M>> {
		if self.is_consistent {
			match self.insert_raw(members).await {
				Ok(_) => Ok(()),
				Err(err) => {
					self.is_consistent = false;
					Err(err)
				}
			}
		} else {
			Err(HotColdError::Irrecoverable)
		}
	}

	/// Inserts members into both Hot and Cold sets and attempts to recover the set if the insertion fails.
	pub async fn rinsert(&mut self, members: Vec<M>) -> Result<(), HotColdError<M>> {
		match self.insert(members).await {
			Ok(_) => Ok(()),
			Err(err) => match err {
				HotColdError::Inconsistent(which, _) => {
					match which {
						InsertionPartial::Hot => {
							self.hot().recover().await.map_err(|_| HotColdError::Irrecoverable)?;
							Ok(())
						}
						InsertionPartial::Both => {
							// try to recover hot then cold
							self.hot().recover().await.map_err(|_| HotColdError::Irrecoverable)?;
							self.cold().recover().await.map_err(|_| HotColdError::Irrecoverable)?;
							Ok(())
						}
					}
				}
				_ => Err(err),
			},
		}
	}
	


}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::collections::HashSet;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct TestMember(pub u8);

    impl TryAsMember for TestMember {
		fn try_as_member(&self) -> Result<Member, MemberError> {
			Ok(Member(vec![self.0]))
		}
	}

	// Reliable Hot set
	pub struct ReliableHot {
		pub cardinality: u64,
		pub ttl: u64,
		pub set: HashSet<Member>,
	}

	pub struct ReliableHotGuard;

	// Implementing the HotGuard trait for ReliableHotGuard
	impl<M> HotGuard<M> for ReliableHotGuard
	where
		M: TryAsMember + Send,
	{
		async fn rollback(&self, _members: &[M]) -> Result<(), HotGuardError> {
			Ok(())  // Always succeed for ReliableHotGuard
		}

		async fn commit(&self, _members: &[M]) -> Result<(), HotGuardError> {
			Ok(())  // Always succeed for ReliableHotGuard
		}
	}

	impl Recoverable for ReliableHot {
		async fn recover(&self) -> Result<(), RecoveryError> {
			Ok(())  // Assume recovery succeeds
		}
	}

	// Implementing the Hot trait for ReliableHot
	impl<M> Hot<M, ReliableHotGuard> for ReliableHot
	where
		M: TryAsMember + Send,
	{
		async fn cardinality(&self) -> Result<u64, HotError> {
			Ok(self.cardinality)
		}

		async fn ttl(&self) -> Result<u64, HotError> {
			Ok(self.ttl)
		}

		async fn prepare_insert(&self, _members: &[M]) -> Result<ReliableHotGuard, HotError> {
			Ok(ReliableHotGuard)  // Always succeed for ReliableHot
		}

		async fn contains(&self, members: &[M]) -> Result<bool, HotError> {
			for member in members {
				let converted_member = member.try_as_member().map_err(|_| HotError::Internal)?;
				if !self.set.contains(&converted_member) {
					return Ok(false);
				}
			}
			Ok(true)
		}

		async fn maybe_contained(&self, members: &[M]) -> Result<bool, HotError> {
			self.contains(members).await
		}

		async fn gc(&self) -> Result<(), HotError> {
			Ok(())  // Assume garbage collection succeeds
		}
	}

	// Reliable Cold set
	pub struct ReliableCold {
		pub set: HashSet<Member>,
	}

	pub struct ReliableColdGuard;

	// Implementing the ColdGuard trait for ReliableColdGuard
	impl<M> ColdGuard<M> for ReliableColdGuard
	where
		M: TryAsMember + Send,
	{
		async fn rollback(&self, _members: &[M]) -> Result<(), ColdGuardError> {
			Ok(())  // Always succeed for ReliableColdGuard
		}

		async fn commit(&self, _members: &[M]) -> Result<(), ColdGuardError> {
			Ok(())  // Always succeed for ReliableColdGuard
		}
	}

	impl Recoverable for ReliableCold {
		async fn recover(&self) -> Result<(), RecoveryError> {
			Ok(())  // Assume recovery succeeds
		}
	}

	// Implementing the Cold trait for ReliableCold
	impl<M> Cold<M, ReliableColdGuard> for ReliableCold
	where
		M: TryAsMember + Send,
	{
		async fn prepare_insert(&self, _members: &[M]) -> Result<ReliableColdGuard, ColdError> {
			Ok(ReliableColdGuard)  // Always succeed for ReliableCold
		}

		async fn contains(&self, members: &[M]) -> Result<bool, ColdError> {
			for member in members {
				let converted_member = member.try_as_member().map_err(|_| ColdError::Internal)?;
				if !self.set.contains(&converted_member) {
					return Ok(false);
				}
			}
			Ok(true)
		}

		async fn maybe_contained(&self, members: &[M]) -> Result<bool, ColdError> {
			self.contains(members).await
		}
	}

    #[tokio::test]
    async fn test_successful_insertion() {
        // Create a reliable Hot set
        let hot = ReliableHot {
            cardinality: 0,
            ttl: 60,
            set: HashSet::new(),
        };

        // Create a reliable Cold set
        let cold = ReliableCold {
            set: HashSet::new(),
        };

        // Create a HotColdSet with the reliable sets
        let mut hot_cold_set = HotColdSet::new(hot, cold);

        // Define some members
        let members = vec![TestMember(1), TestMember(2)];

        // Try inserting into both sets
        let result = hot_cold_set.insert(members.clone()).await;

        // Ensure the insertion was successful
        assert!(result.is_ok(), "Insertion should succeed for reliable Hot and Cold sets.");
    }

}