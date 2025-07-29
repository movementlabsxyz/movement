use crate::garbage::Duration;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

/// A garbage collected map (associative array).
pub struct GcMap<K, V>
where
	K: Eq + Hash + Debug,
	V: Eq + Hash + Debug,
{
	/// The number of some unit time a value is valid for.
	value_ttl: Duration,
	/// The duration of a garbage collection slot in some unit time.
	/// This is used to bin values into slots for O(value_ttl/gc_slot_duration * log value_ttl/gc_slot_duration) garbage collection.
	gc_slot_duration: Duration,
	/// The value lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, HashMap<K, V>>,
}

impl<K, V> GcMap<K, V>
where
	K: Eq + Hash + Debug,
	V: Eq + Hash + Debug,
{
	/// Creates a new GcMap with a specified garbage collection slot duration.
	pub fn new(value_ttl: Duration, gc_slot_duration: Duration) -> Self {
		GcMap { value_ttl, gc_slot_duration, value_lifetimes: BTreeMap::new() }
	}

	/// Gets a value for a key
	pub fn get_value(&self, key: &K) -> Option<&V> {
		// check each slot for the key
		for lifetimes in self.value_lifetimes.values().rev() {
			// reverse order is better average case because highly-used values will be moved up more often
			if let Some(value) = lifetimes.get(key) {
				return Some(value);
			}
		}

		None
	}

	/// Removes the value for an key.
	/// Returns whether the value was removed.
	pub fn remove_value(&mut self, key: &K) -> bool {
		// check each slot for the key
		for lifetimes in self.value_lifetimes.values_mut().rev() {
			if lifetimes.remove(key).is_some() {
				return true;
			}
		}
		false
	}

	/// Sets the value for for a key
	pub fn set_value(&mut self, key: K, value: V, current_time: u64) {
		// remove the old key
		self.remove_value(&key);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time / self.gc_slot_duration.get();

		// add the new value
		self.value_lifetimes.entry(slot).or_insert_with(HashMap::new).insert(key, value);
	}

	/// Garbage collects values that have expired.
	/// This should be called periodically.
	pub fn gc(&mut self, current_time: u64) {
		let gc_slot = current_time / self.gc_slot_duration.get();

		// Calculate the cutoff slot
		let slot_cutoff =
			gc_slot.saturating_sub(self.value_ttl.get() / self.gc_slot_duration.get());

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
		let value_ttl = Duration::try_new(100)?;
		let gc_slot_duration = Duration::try_new(10)?;
		let mut gc_map = GcMap::new(value_ttl, gc_slot_duration);

		let current_time = 0;

		// set the value for key 1
		gc_map.set_value(Key(1), Value(1), current_time);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(1)));

		// overwrite the value for key 1 at the same time
		gc_map.set_value(Key(1), Value(2), current_time);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(2)));

		// overwrite the value for key 1 at a later time
		gc_map.set_value(Key(1), Value(3), current_time + 10);
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(3)));

		// add another key back at the original time
		gc_map.set_value(Key(2), Value(4), current_time);

		// garbage collect
		gc_map.gc(current_time + 100);

		// assert the key 1 is still there
		assert_eq!(gc_map.get_value(&Key(1)), Some(&Value(3)));

		// assert the key 2 is gone
		assert_eq!(gc_map.get_value(&Key(2)), None);

		Ok(())
	}
}
