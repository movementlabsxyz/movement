use crate::garbage::Duration;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

pub struct GcMap<K, V>
where
	K: Eq + Hash + Debug,
	V: Eq + Hash + Debug,
{
	/// The number of milliseconds a value is valid for.
	value_ttl_ms: Duration,
	/// The duration of a garbage collection slot in milliseconds.
	/// This is used to bin values into slots for O(value_ttl_ms/gc_slot_duration_ms * log value_ttl_ms/gc_slot_duration_ms) garbage collection.
	gc_slot_duration_ms: Duration,
	/// The value lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, HashMap<K, V>>,
}

impl<K, V> GcMap<K, V>
where
	K: Eq + Hash + Debug,
	V: Eq + Hash + Debug,
{
	/// Creates a new GcMap with a specified garbage collection slot duration.
	pub fn new(value_ttl_ms: Duration, gc_slot_duration_ms: Duration) -> Self {
		GcMap { value_ttl_ms, gc_slot_duration_ms, value_lifetimes: BTreeMap::new() }
	}

	/// Gets a value for a key
	pub fn get_value(&self, key: &K) -> Option<&V> {
		// check each slot for the key
		for lifetimes in self.value_lifetimes.values().rev() {
			// reverse order is better average case because highly-used values will be moved up more often
			match lifetimes.get(key) {
				Some(value) => {
					// check if the value is still valid
					return Some(value);
				}
				None => {}
			}
		}

		None
	}

	/// Removes the value for an key.
	pub fn remove_value(&mut self, key: &K) {
		// check each slot for the key
		for lifetimes in self.value_lifetimes.values_mut().rev() {
			if lifetimes.remove(key).is_some() {
				break;
			}
		}
	}

	/// Sets the value for for a key
	pub fn set_value(&mut self, key: K, value: V, current_time_ms: u64) {
		// remove the old key
		self.remove_value(&key);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time_ms / self.gc_slot_duration_ms.get();

		// add the new value
		self.value_lifetimes.entry(slot).or_insert_with(HashMap::new).insert(key, value);
	}

	/// Garbage collects values that have expired.
	/// This should be called periodically.
	pub fn gc(&mut self, current_time_ms: u64) {
		let gc_slot = current_time_ms / self.gc_slot_duration_ms.get();

		// Calculate the cutoff slot
		let slot_cutoff = gc_slot - self.value_ttl_ms.get() / self.gc_slot_duration_ms.get();

		let to_keep = self.value_lifetimes.split_off(&(slot_cutoff + 1));

		// Now, `self.value_lifetimes` contains only entries with keys < `slot_cutoff`.
		// Reassign `self.value_lifetimes` to `to_keep` to keep only entries >= `slot_cutoff`.
		self.value_lifetimes = to_keep;
	}
}

#[cfg(test)]
pub mod test {

	use super::*;

	#[derive(Debug, Eq, PartialEq, Hash)]
	pub struct Key(u64);

	#[derive(Debug, Eq, PartialEq, Hash)]
	pub struct Value(u64);

	#[test]
	fn test_gc_map() -> Result<(), anyhow::Error> {
		let value_ttl_ms = Duration::try_new(100)?;
		let gc_slot_duration_ms = Duration::try_new(10)?;
		let mut gc_map = GcMap::new(value_ttl_ms, gc_slot_duration_ms);

		let current_time_ms = 0;

		// set the value for key 1
		gc_map.set_value(Key(1), Value(1), current_time_ms);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(1)));

		// overwrite the value for key 1 at the same time
		gc_map.set_value(Key(1), Value(2), current_time_ms);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(2)));

		// overwrite the value for key 1 at a later time
		gc_map.set_value(Key(1), Value(3), current_time_ms + 10);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(3)));

		// add another key back at the original time
		gc_map.set_value(Key(2), Value(4), current_time_ms);

		// garbage collect
		gc_map.gc(current_time_ms + 100);

		// assert the key 1 is still there
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(3)));

		// assert the key 2 is gone
		assert_eq!(gc_map.get_value(&Key(2)), None);

		Ok(())
	}
}
