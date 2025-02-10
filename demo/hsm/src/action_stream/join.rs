use crate::{ActionStream, Message};

/// Joins several streams together.
/// Notifies all streams of messages emitted from elsewhere in the system.
/// Round-robins through streams for next.
pub struct Join {
	streams: Vec<Box<dyn ActionStream + Send>>,
	current: usize,
}

impl Join {
	/// Creates a new `Join` stream.
	pub fn new(streams: Vec<Box<dyn ActionStream + Send>>) -> Self {
		Self { streams, current: 0 }
	}
}

#[async_trait::async_trait]
impl ActionStream for Join {
	/// Notifies the stream of a message emitted from elsewhere in the system.
	async fn notify(&mut self, message: Message) -> Result<(), anyhow::Error> {
		for stream in &mut self.streams {
			stream.notify(message.clone()).await?;
		}
		Ok(())
	}

	/// Gets the message to act upon.
	async fn next(&mut self) -> Result<Option<Message>, anyhow::Error> {
		let mut next = None;
		for _ in 0..self.streams.len() {
			let stream = &mut self.streams[self.current];
			next = stream.next().await?;
			self.current = (self.current + 1) % self.streams.len();
			if next.is_some() {
				break;
			}
		}
		Ok(next)
	}
}
