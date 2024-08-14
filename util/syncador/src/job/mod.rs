use futures::stream;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use tokio::time::interval;

pub struct Event {
	stream: Pin<Box<dyn Stream<Item = ()> + Send>>,
}

impl Event {
	pub fn new<S>(stream: S) -> Self
	where
		S: Stream<Item = ()> + Send + 'static,
	{
		Event { stream: Box::pin(stream) }
	}

	pub fn once() -> Self {
		Event { stream: Box::pin(stream::once(async { () })) }
	}

	pub fn every(duration: std::time::Duration) -> Self {
		Event {
			stream: async_stream::stream! {
				let mut interval = interval(duration);
				loop {
					interval.tick().await;
					yield ();
				}
			}
			.boxed(),
		}
	}

	pub fn success(
		async_fn: impl std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
	) -> Self {
		Event {
			stream: async_stream::stream! {
				match async_fn.await {
					Ok(_) => yield (),
					Err(e) => { },
				}
			}
			.boxed(),
		}
	}

	pub async fn next(&mut self) -> Option<()> {
		self.stream.as_mut().next().await
	}
}
