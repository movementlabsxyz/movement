use crate::garbage::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GcCounter {
	/// The number of milliseconds a value is valid for.
	value_ttl_ms: Duration,
	/// The duration of a garbage collection slot in milliseconds.
	gc_slot_duration_ms: Duration,
	/// The array of atomic counters for value lifetimes, where each entry represents a slot with a timestamp and count.
	value_lifetimes: Arc<Vec<(AtomicU64, AtomicU64)>>,
	/// The number of slots calculated as value_ttl_ms / gc_slot_duration_ms.
	num_slots: u64,
}

impl GcCounter {
	/// Creates a new GcCounter with a specified garbage collection slot duration.
	pub fn new(value_ttl_ms: Duration, gc_slot_duration_ms: Duration) -> Self {
		let num_slots = value_ttl_ms.get() / gc_slot_duration_ms.get();
		let value_lifetimes =
			Arc::new((0..num_slots).map(|_| (AtomicU64::new(0), AtomicU64::new(0))).collect());
		GcCounter { value_ttl_ms, gc_slot_duration_ms, value_lifetimes, num_slots }
	}

	/// Decrements the value, saturating over non-zero slots.
	pub fn decrement(&self, mut amount: u64) {
		for (_, count) in self.value_lifetimes.iter() {
			// Use `fetch_update` to perform a safe, atomic update
			let result = count.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current_count| {
				if current_count == 0 {
					None // Stop if the count is already zero
				} else if current_count >= amount {
					Some(current_count - amount) // Deduct the full amount
				} else {
					// Otherwise, subtract what we can and let `amount` carry the rest
					amount -= current_count;
					Some(0)
				}
			});

			// If the update was successful or if the slot is zero, we can move on
			if result.is_ok() || amount == 0 {
				break;
			}

			// Stop early if the remaining amount has been fully decremented
			if amount == 0 {
				break;
			}
		}
	}

	/// Increments the value in a specific slot.
	pub fn increment(&self, current_time_ms: u64, amount: u64) {
		let slot_timestamp = current_time_ms / self.gc_slot_duration_ms.get();
		let slot = slot_timestamp % self.num_slots;
		let (active_slot_timestamp, count) = &self.value_lifetimes[slot as usize];

		// Atomically check and set the timestamp if it doesn't match current_time_ms
		let active_slot = active_slot_timestamp.load(Ordering::Relaxed);
		if active_slot == slot {
			// Same timestamp, increment count
			count.fetch_add(amount, Ordering::SeqCst);
		} else {
			// Different timestamp, reset slot for new period
			active_slot_timestamp.store(slot_timestamp, Ordering::SeqCst);
			count.store(amount, Ordering::SeqCst);
		}
	}

	/// Gets the current count across all slots
	pub fn get_count(&self) -> u64 {
		self.value_lifetimes
			.iter()
			.map(|(_, count)| count.load(Ordering::Relaxed))
			.sum()
	}

	/// Garbage collects values that have expired.
	/// This should be called periodically.
	pub fn gc(&self, current_time_ms: u64) {
		let cutoff_time = current_time_ms - self.value_ttl_ms.get();

		for (slot_timestamp, count) in self.value_lifetimes.iter() {
			// If the timestamp is older than the cutoff, reset the slot
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
		let value_ttl_ms = Duration::try_new(100)?;
		let gc_slot_duration_ms = Duration::try_new(10)?;
		let gc_counter = GcCounter::new(value_ttl_ms, gc_slot_duration_ms);

		let current_time_ms = 0;

		// add three
		gc_counter.increment(current_time_ms, 1);
		gc_counter.increment(current_time_ms, 1);
		gc_counter.increment(current_time_ms, 1);
		assert_eq!(gc_counter.get_count(), 3);

		// decrement one
		gc_counter.decrement(1);
		assert_eq!(gc_counter.get_count(), 2);

		// add one garbage collect the rest
		gc_counter.increment(current_time_ms + 10, 1);
		gc_counter.gc(current_time_ms + 100);

		// check that the count is 1
		assert_eq!(gc_counter.get_count(), 1);

		Ok(())
	}

	#[test]
	fn test_multiple_references() -> Result<(), anyhow::Error> {
		let value_ttl_ms = Duration::try_new(100)?;
		let gc_slot_duration_ms = Duration::try_new(10)?;
		let gc_counter = GcCounter::new(value_ttl_ms, gc_slot_duration_ms);
		let gc_counter_clone = gc_counter.clone();

		let current_time_ms = 0;

		// add three
		gc_counter.increment(current_time_ms, 1);
		gc_counter_clone.increment(current_time_ms, 1);
		gc_counter.increment(current_time_ms, 1);
		assert_eq!(gc_counter.get_count(), 3);

		// decrement one
		gc_counter.decrement(1);
		assert_eq!(gc_counter.get_count(), 2);

		// add one garbage collect the rest
		gc_counter_clone.increment(current_time_ms + 10, 1);
		gc_counter.gc(current_time_ms + 100);

		// check that the count is 1
		assert_eq!(gc_counter_clone.get_count(), 1);

		Ok(())
	}
}
