use crate::garbage::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// An atomic Garbage Collected counter.
/// The total count accumulated can be garbage collected periodically using the `gc` method.
/// Counts by slot are stored in a ring buffer. So, reused slots will also be garbage collected when they are reassigned.
/// The reusage of slots does not remove the need to call `gc` periodically, as slots which are not reused would not be garbage collected, causing the count to drift.
/// Insofare as `gc` is called at least once per ttl, this will safely garbage collect.
#[derive(Debug, Clone)]
pub struct GcCounter {
	/// The number of some unit time a value is valid for.
	value_ttl: Duration,
	/// The duration of a garbage collection slot in some unit time.
	gc_slot_duration: Duration,
	/// The array of atomic counters for value lifetimes, where each entry represents a slot with a timestamp and count.
	value_lifetimes: Arc<Vec<(AtomicU64, AtomicU64)>>,
	/// The number of slots calculated as value_ttl / gc_slot_duration.
	num_slots: u64,
}

impl GcCounter {
	/// Creates a new GcCounter with a specified garbage collection slot duration.
	pub fn new(value_ttl: Duration, gc_slot_duration: Duration) -> Self {
		let num_slots = 2 * (value_ttl.get() / gc_slot_duration.get()); // Double the number of slots
		let value_lifetimes =
			Arc::new((0..num_slots).map(|_| (AtomicU64::new(0), AtomicU64::new(0))).collect());
		GcCounter { value_ttl, gc_slot_duration, value_lifetimes, num_slots }
	}

	/// Decrements the value, saturating over non-zero slots.
	pub fn decrement(&self, mut amount: u64) {
		for (_, count) in self.value_lifetimes.iter() {
			let result = count.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current_count| {
				if current_count == 0 {
					None // Stop if the count is already zero
				} else if current_count >= amount {
					Some(current_count - amount) // Deduct the full amount
				} else {
					amount -= current_count;
					Some(0)
				}
			});

			if result.is_ok() || amount == 0 {
				break;
			}

			if amount == 0 {
				break;
			}
		}
	}

	/// Increments the value in a specific slot.
	pub fn increment(&self, current_time: u64, amount: u64) {
		let slot_timestamp = current_time / self.gc_slot_duration.get();
		let pass_number = slot_timestamp / (self.num_slots / 2); // Determine if we're in the first or second pass
		let base_slot = slot_timestamp % (self.num_slots / 2); // Calculate the base slot index within the first pass

		// Calculate the actual slot index by adding the pass offset (0 or 1 times num_slots / 2)
		let slot = base_slot + (pass_number % 2) * (self.num_slots / 2);
		let (active_slot_timestamp, count) = &self.value_lifetimes[slot as usize];

		let active_slot = active_slot_timestamp.load(Ordering::Relaxed);
		let active_amount = count.load(Ordering::Relaxed);

		if active_slot == slot_timestamp {
			count.fetch_add(amount, Ordering::SeqCst);
		} else {
			if active_slot_timestamp
				.compare_exchange(active_slot, slot_timestamp, Ordering::SeqCst, Ordering::Relaxed)
				.is_ok()
			{
				if !count
					.compare_exchange(active_amount, amount, Ordering::SeqCst, Ordering::Relaxed)
					.is_ok()
				{
					count.fetch_add(amount, Ordering::SeqCst);
				}
			} else {
				count.fetch_add(amount, Ordering::SeqCst);
			}
		}
	}

	/// Gets the current count across all slots.
	pub fn get_count(&self) -> u64 {
		self.value_lifetimes
			.iter()
			.map(|(_, count)| count.load(Ordering::Relaxed))
			.sum()
	}

	/// Garbage collects values that have expired.
	pub fn gc(&self, current_time: u64) {
		let cutoff_time = current_time - self.value_ttl.get();

		for (slot_timestamp, count) in self.value_lifetimes.iter() {
			if slot_timestamp.load(Ordering::Relaxed) <= cutoff_time {
				slot_timestamp.store(0, Ordering::SeqCst);
				count.store(0, Ordering::SeqCst);
			}
		}
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;

	#[test]
	fn test_gc_counter() -> Result<(), anyhow::Error> {
		let value_ttl = Duration::try_new(100)?;
		let gc_slot_duration = Duration::try_new(10)?;
		let gc_counter = GcCounter::new(value_ttl, gc_slot_duration);

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

	#[test]
	fn test_multiple_references() -> Result<(), anyhow::Error> {
		let value_ttl = Duration::try_new(100)?;
		let gc_slot_duration = Duration::try_new(10)?;
		let gc_counter = GcCounter::new(value_ttl, gc_slot_duration);
		let gc_counter_clone = gc_counter.clone();

		let current_time = 0;

		// add three
		gc_counter.increment(current_time, 1);
		gc_counter_clone.increment(current_time, 1);
		gc_counter.increment(current_time, 1);
		assert_eq!(gc_counter.get_count(), 3);

		// decrement one
		gc_counter.decrement(1);
		assert_eq!(gc_counter.get_count(), 2);

		// add one garbage collect the rest
		gc_counter_clone.increment(current_time + 10, 1);
		gc_counter.gc(current_time + 100);

		// check that the count is 1
		assert_eq!(gc_counter_clone.get_count(), 1);

		Ok(())
	}
}
