use rand::{rngs::StdRng, SeedableRng};
use rand::{Rng, RngCore};
use rand_chacha::ChaChaRng;

pub type TestRng = StdRng;

pub trait RngSeededClone: Rng + SeedableRng {
	fn seeded_clone(&mut self) -> Self;
}

impl RngSeededClone for StdRng {
	fn seeded_clone(&mut self) -> Self {
		self.clone()
	}
}

impl RngSeededClone for ChaChaRng {
	fn seeded_clone(&mut self) -> Self {
		let mut seed = [0u8; 32];
		self.fill_bytes(&mut seed);
		ChaChaRng::from_seed(seed)
	}
}
