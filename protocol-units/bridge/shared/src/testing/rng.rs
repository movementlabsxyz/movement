use rand::SeedableRng;
use rand::{Rng, RngCore};
use rand_chacha::ChaChaRng;

pub type TestRng = ChaChaRng;

pub trait RngSeededClone: Rng + SeedableRng {
	fn seeded_clone(&mut self) -> Self;
}

impl RngSeededClone for ChaChaRng {
	fn seeded_clone(&mut self) -> Self {
		let mut seed = [0u8; 32];
		self.fill_bytes(&mut seed);
		ChaChaRng::from_seed(seed)
	}
}
