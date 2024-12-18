use crate::{ActionStream, Bytes, Message};
use rand::{Rng, RngCore};

/// A stream of random messages.
pub struct Random;

#[async_trait::async_trait]
impl ActionStream for Random {
	/// Notifies the stream of a message emitted from elsewhere in the system.
	async fn notify(&mut self, _message: Message) -> Result<(), anyhow::Error> {
		Ok(())
	}

	/// Gets the message to act upon.
	async fn next(&mut self) -> Result<Option<Message>, anyhow::Error> {
		// Generate a random vec of bytes
		let mut rng = rand::thread_rng();
		let len = rng.gen_range(32, 256);
		let mut bytes = vec![0u8; len];
		rng.fill_bytes(&mut bytes);

		Ok(Some(Message::Sign(Bytes(bytes))))
	}
}
