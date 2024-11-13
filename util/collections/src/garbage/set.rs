use crate::garbage::Duration;
use std::collections::{BTreeMap, HashSet};
use std::hash::Hash;

pub struct GcSet<V>
where
	V: Eq + Hash,
{
	/// The number of some unit time a value is valid for.
	value_ttl: Duration,
	/// The duration of a garbage collection slot in some unit time.
	/// This is used to bin values into slots for O(value_ttl/gc_slot_duration * log value_ttl/gc_slot_duration) garbage collection.
	gc_slot_duration: Duration,
	/// The value lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, HashSet<V>>,
}

impl<V> GcSet<V>
where
	V: Eq + Hash,
{
	/// Creates a new GcSet with a specified garbage collection slot duration.
	pub fn new(value_ttl: Duration, gc_slot_duration: Duration) -> Self {
		GcSet { value_ttl, gc_slot_duration, value_lifetimes: BTreeMap::new() }
	}

	/// Removes the value for an key.
	pub fn remove_value(&mut self, value: &V) {
		// check each slot for the key
		for lifetimes in self.value_lifetimes.values_mut().rev() {
			if lifetimes.remove(value) {
				break;
			}
		}
	}

	/// Sets the value for an key.
	pub fn insert(&mut self, value: V, current_time: u64) {
		// remove the old value
		self.remove_value(&value);

		// compute the slot for the new lifetime and add accordingly
		let slot = current_time / self.gc_slot_duration.get();

		// add the new value
		self.value_lifetimes.entry(slot).or_insert_with(HashSet::new).insert(value);
	}

	/// Checks if the value is in the set.
	pub fn contains(&self, value: &V) -> bool {
		self.value_lifetimes.values().any(|lifetimes| lifetimes.contains(value))
	}

	/// Garbage collects values that have expired.
	/// This should be called periodically.
	pub fn gc(&mut self, current_time: u64) {
		let gc_slot = current_time / self.gc_slot_duration.get();

		// remove all slots that are too old
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
	pub struct Value(u64);

	#[test]
	fn test_gc_set() -> Result<(), anyhow::Error> {
		let value_ttl = Duration::try_new(100)?;
		let gc_slot_duration = Duration::try_new(10)?;
		let mut gc_set = GcSet::new(value_ttl, gc_slot_duration);

		let current_time = 0;

		// set the value for key 1
		gc_set.insert(Value(1), current_time);
		assert_eq!(gc_set.contains(&Value(1)), true);

		// write the value for key 1 again at later time
		gc_set.insert(Value(1), current_time + 100);
		assert_eq!(gc_set.contains(&Value(1)), true);

		// add another value back at the original time
		gc_set.insert(Value(2), current_time);

		// garbage collect
		gc_set.gc(current_time + 100);

		// assert the value 1 is still there
		assert_eq!(gc_set.contains(&Value(1)), true);

		// assert the value 2 is gone
		assert_eq!(gc_set.contains(&Value(2)), false);

		Ok(())
	}
}
