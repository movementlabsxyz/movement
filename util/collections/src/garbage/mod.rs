#[cfg(feature = "unstable")]
pub mod atomic;
pub mod counted;
pub mod map;
pub mod set;
use std::num::NonZeroU64;

#[derive(Debug, Clone)]
pub struct Duration(NonZeroU64);

impl Duration {
	pub fn try_new(value: u64) -> Result<Self, anyhow::Error> {
		Ok(Duration(
			NonZeroU64::new(value).ok_or_else(|| anyhow::anyhow!("Duration must be non-zero"))?,
		))
	}

	pub fn get(&self) -> u64 {
		self.0.get()
	}
}
