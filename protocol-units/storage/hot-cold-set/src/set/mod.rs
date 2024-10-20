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

/// A member of the set is represented as a byte array.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Member(pub Vec<u8>);

/// Error types for the Hot guard.
#[derive(Debug, Error)]
pub enum HotGuardError {
	#[error("hot guard failed to rollback the insertion")]
	FailedToRollback,
	#[error("hot guard failed to commit the insertion")]
	FailedToCommit,
}

/// A Hot guard responsible for rollback or commit operations.
pub trait HotGuard<M> where M: TryInto<Member> {
	/// Rollback the prepared insertion in the Hot set.
	async fn rollback(&self, members: &[M]) -> Result<(), HotGuardError>;

	/// Commit the prepared insertion in the Hot set.
	async fn commit(&self, members: &[M]) -> Result<(), HotGuardError>;
}

#[derive(Debug, Error)]
pub enum HotError {}

/// The Hot portion of the set, optimized for fast access and size.
/// The Hot set is garbage collected and typically uses structures like Bloom filters to probabilistically check membership.
/// The Hot set supports asynchronous operations and should implement this trait with a type-specific guard.
pub trait Hot<M, G>
where
	M: TryInto<Member>,
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
pub trait ColdGuard<M> where M: TryInto<Member> {
	/// Rollback the prepared insertion in the Cold set.
	async fn rollback(&self, members: &[M]) -> Result<(), ColdGuardError>;

	/// Commit the prepared insertion in the Cold set.
	async fn commit(&self, members: &[M]) -> Result<(), ColdGuardError>;
}

#[derive(Debug, Error)]
pub enum ColdError {}

/// The Cold portion of the set is append-only and intended for long-term storage.
/// The Cold set serves as a backup and is never garbage collected by the application.
pub trait Cold<M, G>
where
	M: TryInto<Member>,
	G: ColdGuard<M>,
{
	/// Prepare to insert a collection of members into the Hot set.
    async fn prepare_insert(&self, members: &[M]) -> Result<G, HotError>;

    /// Check if a collection of members is in the Hot set.
    async fn contains(&self, members: &[M]) -> Result<bool, HotError>;

    /// Check if the Hot set likely contained a collection of members.
    async fn maybe_contained(&self, members: &[M]) -> Result<bool, HotError>;

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
	M: TryInto<Member>,
{
	#[error("hot-cold set is inconsistent (partially committed)")]
	Inconsistent(InsertionPartial, Vec<M>),
	#[error("failed to insert member")]
	FailedToInsert,
}

/// The `HotColdSet` struct ensures synchronized inclusion of members between the Hot and Cold sets.
pub struct HotColdSet<M, HG, CG, H, C>
where
	M: TryInto<Member>,
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
}

impl<M, HG, CG, H, C> HotColdSet<M, HG, CG, H, C>
where
	M: TryInto<Member>,
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
	pub async fn insert<I>(&self, members: Vec<M>) -> Result<(), HotColdError<M>> {
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
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::collections::HashSet;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct TestMember(u8);

    impl TryInto<Member> for TestMember {

        type Error = ();

        fn try_into(self) -> Result<Member, ()> {
            Ok(Member(vec![self.0]))
        }
    }

    pub struct ReliableHot {
        cardinality: u64,
        ttl: u64,
        set: HashSet<Member>,
    }

    pub struct ReliableHotGuard;

    pub struct ReliableCold {
        set: HashSet<Member>,
    }

    pub struct ReliableColdGuard;

    pub struct UnreliableCommitHot {
        cardinality: u64,
        ttl: u64,
        set: HashSet<Member>,
    }

    pub struct UnreliableCommitHotGuard;

    pub struct UnreliableCommitCold {
        set: HashSet<Member>,
    }

    pub struct UnreliableCommitColdGuard;

    pub struct UnreliableRollbackHot {
        cardinality: u64,
        ttl: u64,
        set: HashSet<Member>,
    }

    pub struct UnreliableRollbackHotGuard;

    pub struct UnreliableRollbackCold {
        set: HashSet<Member>,
    }

    pub struct UnreliableRollbackColdGuard;

}