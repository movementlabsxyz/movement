use crate::{ActionStream, Message};
use std::collections::VecDeque;

/// Adds all verify messages of which the stream is notified back to the stream
pub struct NotifyVerify {
	buffer: VecDeque<Message>,
}

impl NotifyVerify {
	/// Creates a new `NotifyVerify` stream.
	pub fn new() -> Self {
		Self { buffer: VecDeque::new() }
	}
}

#[async_trait::async_trait]
impl ActionStream for NotifyVerify {
	/// Notifies the stream of a message emitted from elsewhere in the system.
	async fn notify(&mut self, message: Message) -> Result<(), anyhow::Error> {
		self.buffer.push_back(message);
		Ok(())
	}

	/// Gets the message to act upon.
	async fn next(&mut self) -> Result<Option<Message>, anyhow::Error> {
		Ok(self.buffer.pop_front())
	}
}
