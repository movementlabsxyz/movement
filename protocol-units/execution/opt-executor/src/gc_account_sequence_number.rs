use aptos_types::account_address::AccountAddress;
use std::collections::{BTreeMap, HashMap};
use tracing::info;

pub struct UsedSequenceNumberPool {
	/// The number of milliseconds a sequence number is valid for.
	sequence_number_ttl_ms: u64,
	/// The duration of a garbage collection slot in milliseconds.
	/// This is used to bin sequence numbers into slots for O(sequence_number_ttl_ms/gc_slot_duration_ms * log sequence_number_ttl_ms/gc_slot_duration_ms) garbage collection.
	gc_slot_duration_ms: u64,
	/// The sequence number lifetimes, indexed by slot.
	sequence_number_lifetimes: BTreeMap<u64, HashMap<AccountAddress, u64>>,
}

impl UsedSequenceNumberPool {
	/// Creates a new UsedSequenceNumberPool with a specified garbage collection slot duration.
	pub(crate) fn new(sequence_number_ttl_ms: u64, gc_slot_duration_ms: u64) -> Self {
		UsedSequenceNumberPool {
			sequence_number_ttl_ms,
			gc_slot_duration_ms,
			sequence_number_lifetimes: BTreeMap::new(),
		}
	}

	/// Gets a sequence number for an account
	pub(crate) fn get_sequence_number(&self, account: &AccountAddress) -> Option<u64> {
		// check each slot for the account
		for lifetimes in self.sequence_number_lifetimes.values().rev() {
			// reverse order is better average case because highly-used sequence numbers will be moved up more often
			match lifetimes.get(account) {
				Some(sequence_number) => {
					// check if the sequence number is still valid
					return Some(*sequence_number);
				}
				None => {}
			}
		}

		None
	}

	/// Removes the sequence number for an account.
	pub(crate) fn remove_sequence_number(&mut self, account_address: &AccountAddress) {
		// check each slot for the account
		for lifetimes in self.sequence_number_lifetimes.values_mut().rev() {
			if lifetimes.remove(account_address).is_some() {
				break;
			}
		}
	}

	/// Sets the sequence number for an account.
	pub(crate) fn set_sequence_number(
		&mut self,
		account_address: &AccountAddress,
		sequence_number: u64,
		current_time_ms: u64,
	) {
		// remove the old sequence number
		self.remove_sequence_number(account_address);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time_ms / self.gc_slot_duration_ms;

		// add the new sequence number
		self.sequence_number_lifetimes
			.entry(slot)
			.or_insert_with(HashMap::new)
			.insert(*account_address, sequence_number);
	}

	/// Garbage collects sequence numbers that have expired.
	/// This should be called periodically.
	pub(crate) fn gc(&mut self, current_time_ms: u64) {
		let gc_slot = current_time_ms / self.gc_slot_duration_ms;

		// remove all slots that are too old
		let slot_cutoff = gc_slot - self.sequence_number_ttl_ms / self.gc_slot_duration_ms;
		let slots_to_remove: Vec<u64> = self
			.sequence_number_lifetimes
			.keys()
			.take_while(|slot| **slot < slot_cutoff)
			.cloned()
			.collect();
		for slot in slots_to_remove {
			println!(
				"Garbage collecting sequence number slot {} with duration {} timestamp {}",
				slot,
				self.gc_slot_duration_ms,
				slot * self.gc_slot_duration_ms
			);
			self.sequence_number_lifetimes.remove(&slot);
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::*;

	#[test]
	fn test_inserts() {
		let mut pool = UsedSequenceNumberPool::new(1000, 100);
		let account1 = AccountAddress::random();
		let account2 = AccountAddress::random();

		pool.set_sequence_number(&account1, 1, 0);
		pool.set_sequence_number(&account2, 2, 0);
		assert_eq!(pool.get_sequence_number(&account1), Some(1));
		assert_eq!(pool.get_sequence_number(&account2), Some(2));
	}

	#[test]
	fn test_removes() {
		let mut pool = UsedSequenceNumberPool::new(1000, 100);
		let account1 = AccountAddress::random();
		let account2 = AccountAddress::random();

		pool.set_sequence_number(&account1, 1, 0);
		pool.set_sequence_number(&account2, 2, 0);
		pool.remove_sequence_number(&account1);
		assert_eq!(pool.get_sequence_number(&account1), None);
		assert_eq!(pool.get_sequence_number(&account2), Some(2));
	}

	#[test]
	fn test_gc() {
		let mut pool = UsedSequenceNumberPool::new(1000, 100);
		let account1 = AccountAddress::random();
		let account2 = AccountAddress::random();

		pool.set_sequence_number(&account1, 1, 0);
		pool.set_sequence_number(&account2, 2, 0);
		pool.gc(1000);
		assert_eq!(pool.get_sequence_number(&account1), Some(1));
		assert_eq!(pool.get_sequence_number(&account2), Some(2));
		pool.gc(2000);
		assert_eq!(pool.get_sequence_number(&account1), None);
		assert_eq!(pool.get_sequence_number(&account2), None);
	}

	#[test]
	fn test_gc_removes_some_not_all() {
		let mut pool = UsedSequenceNumberPool::new(1000, 100);
		let account1 = AccountAddress::random();
		let account2 = AccountAddress::random();

		pool.set_sequence_number(&account1, 1, 0);
		pool.set_sequence_number(&account2, 2, 0);
		pool.gc(1000);
		assert_eq!(pool.get_sequence_number(&account1), Some(1));
		assert_eq!(pool.get_sequence_number(&account2), Some(2));
		pool.set_sequence_number(&account1, 3, 1000);
		pool.gc(2000);
		assert_eq!(pool.get_sequence_number(&account1), Some(3));
		assert_eq!(pool.get_sequence_number(&account2), None);
	}
}
