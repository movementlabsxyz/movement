use crate::garbage::Duration;
use std::collections::BTreeMap;

pub struct GcCounter {
	/// The number of some unit time a value is valid for.
	value_ttl: Duration,
	/// The duration of a garbage collection slot in some unit time.
	/// This is used to bin values into slots for O(value_ttl/gc_slot_duration * log value_ttl/gc_slot_duration) garbage collection.
	gc_slot_duration: Duration,
	/// The value lifetimes, indexed by slot.
	value_lifetimes: BTreeMap<u64, u64>,
}

impl GcCounter {
	/// Creates a new GcCounter with a specified garbage collection slot duration.
	pub fn new(value_ttl: Duration, gc_slot_duration: Duration) -> Self {
		GcCounter { value_ttl, gc_slot_duration, value_lifetimes: BTreeMap::new() }
	}

	/// Decrements from the first slot that has a non-zero value, saturating at zero.
	pub fn decrement(&mut self, mut value: u64) {
		// Iterate over each slot
		for lifetime in self.value_lifetimes.values_mut() {
			if *lifetime > 0 {
				// Determine how much to decrement, without going below zero
				let decrement_amount = value.min(*lifetime);
				*lifetime -= decrement_amount;
				// Reduce the remaining amount by what was actually decremented
				value -= decrement_amount;

				// If there's no residual value to decrement, we are done
				if value == 0 {
					break;
				}
			}
		}
	}

	/// Sets the value for an key.
	pub fn increment(&mut self, current_time: u64, value: u64) {
		// compute the slot for the new lifetime and add accordingly
		let slot = current_time / self.gc_slot_duration.get();

		// increment the slot
		match self.value_lifetimes.get_mut(&slot) {
			Some(lifetime) => {
				*lifetime += value;
			}
			None => {
				self.value_lifetimes.insert(slot, value);
			}
		}
	}

	/// Gets the current count
	pub fn get_count(&self) -> u64 {
		// sum up all the slots
		self.value_lifetimes.values().sum()
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
pub mod tests {
	use super::*;

	#[test]
	fn test_gc_counter() -> Result<(), anyhow::Error> {
		let value_ttl = Duration::try_new(100)?;
		let gc_slot_duration = Duration::try_new(10)?;
		let mut gc_counter = GcCounter::new(value_ttl, gc_slot_duration);

		let current_time = 0;

		// add three
		gc_counter.increment(current_time, 1);
		gc_counter.increment(current_time, 1);
		gc_counter.increment(current_time, 1);
		assert_eq!(gc_counter.get_count(), 3);

		// decrement one
		gc_counter.decrement(1);
		assert_eq!(gc_counter.get_count(), 2);

		// add one garbage collect the rest
		gc_counter.increment(current_time + 10, 1);
		gc_counter.gc(current_time + 100);

		// check that the count is 1
		assert_eq!(gc_counter.get_count(), 1);

		Ok(())
	}
}
