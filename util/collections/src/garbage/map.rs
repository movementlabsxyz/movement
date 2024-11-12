use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

pub struct GcMap<K, V>
where
	K: Eq + Hash,
	V: Eq + Hash,
{
	/// The number of milliseconds a sequence number is valid for.
	value_ttl_ms: u64,
	/// The duration of a garbage collection slot in milliseconds.
	/// This is used to bin sequence numbers into slots for O(value_ttl_ms/gc_slot_duration_ms * log value_ttl_ms/gc_slot_duration_ms) garbage collection.
	gc_slot_duration_ms: u64,
	/// The sequence number lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, HashMap<K, V>>,
}

impl<K, V> GcMap<K, V>
where
	K: Eq + Hash,
	V: Eq + Hash,
{
	/// Creates a new GcMap with a specified garbage collection slot duration.
	pub fn new(value_ttl_ms: u64, gc_slot_duration_ms: u64) -> Self {
		GcMap { value_ttl_ms, gc_slot_duration_ms, value_lifetimes: BTreeMap::new() }
	}

	/// Gets a sequence number for an account
	pub fn get_value(&self, account: &K) -> Option<&V> {
		// check each slot for the account
		for lifetimes in self.value_lifetimes.values().rev() {
			// reverse order is better average case because highly-used sequence numbers will be moved up more often
			match lifetimes.get(account) {
				Some(value) => {
					// check if the sequence number is still valid
					return Some(value);
				}
				None => {}
			}
		}

		None
	}

	/// Removes the sequence number for an account.
	pub fn remove_value(&mut self, key: &K) {
		// check each slot for the account
		for lifetimes in self.value_lifetimes.values_mut().rev() {
			if lifetimes.remove(key).is_some() {
				break;
			}
		}
	}

	/// Sets the sequence number for an account.
	pub fn set_value(&mut self, key: K, value: V, current_time_ms: u64) {
		// remove the old sequence number
		self.remove_value(&key);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time_ms / self.gc_slot_duration_ms;

		// add the new sequence number
		self.value_lifetimes.entry(slot).or_insert_with(HashMap::new).insert(key, value);
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
