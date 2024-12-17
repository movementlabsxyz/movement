use crate::{ActionStream, Bytes, Message};
use rand::{Rng, RngCore};

/// A stream of random messages.
pub struct Random;

#[async_trait::async_trait]
impl ActionStream for Random {
	async fn next(&mut self) -> Option<Message> {
		// Generate a random vec of bytes
		let mut rng = rand::thread_rng();
		let len = rng.gen_range(1, 10);
		let mut bytes = vec![0u8; len];
		rng.fill_bytes(&mut bytes);

		Some(Message::Sign(Bytes(bytes)))
	}
}
