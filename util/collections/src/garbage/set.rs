use std::collections::{BTreeMap, HashSet};
use std::hash::Hash;

pub struct GcMap<V>
where
	V: Eq + Hash,
{
	/// The number of milliseconds a sequence number is valid for.
	value_ttl_ms: u64,
	/// The duration of a garbage collection slot in milliseconds.
	/// This is used to bin sequence numbers into slots for O(value_ttl_ms/gc_slot_duration_ms * log value_ttl_ms/gc_slot_duration_ms) garbage collection.
	gc_slot_duration_ms: u64,
	/// The sequence number lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, HashSet<V>>,
}

impl<V> GcMap<V>
where
	V: Eq + Hash,
{
	/// Creates a new GcMap with a specified garbage collection slot duration.
	pub fn new(value_ttl_ms: u64, gc_slot_duration_ms: u64) -> Self {
		GcMap { value_ttl_ms, gc_slot_duration_ms, value_lifetimes: BTreeMap::new() }
	}

	/// Removes the sequence number for an account.
	pub fn remove_value(&mut self, value: &V) {
		// check each slot for the account
		for lifetimes in self.value_lifetimes.values_mut().rev() {
			lifetimes.remove(value);
		}
	}

	/// Sets the sequence number for an account.
	pub fn insert(&mut self, value: V, current_time_ms: u64) {
		// remove the old sequence number
		self.remove_value(&value);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time_ms / self.gc_slot_duration_ms;

		// add the new sequence number
		self.value_lifetimes.entry(slot).or_insert_with(HashSet::new).insert(value);
	}

	/// Garbage collects sequence numbers that have expired.
	/// This should be called periodically.
	pub fn gc(&mut self, current_time_ms: u64) {
		let gc_slot = current_time_ms / self.gc_slot_duration_ms;

		// remove all slots that are too old
		let slot_cutoff = gc_slot - self.value_ttl_ms / self.gc_slot_duration_ms;
		let slots_to_remove: Vec<u64> = self
			.value_lifetimes
			.keys()
			.take_while(|slot| **slot < slot_cutoff)
			.cloned()
			.collect();
		for slot in slots_to_remove {
			self.value_lifetimes.remove(&slot);
		}
	}
}
