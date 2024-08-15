pub mod backend;
pub mod files;

pub use crate::backend::{PullOperations, PushOperations};
pub use crate::files::package::{Package, PackageElement};
use futures::pin_mut;
use futures::stream::{Stream, StreamExt};

pub async fn sync<
	Q: PushOperations + Send + Sync + 'static,
	P: PullOperations + Send + Sync + 'static,
>(
	push_stream: impl Stream<Item = Result<Package, anyhow::Error>> + Send,
	push: Q,
	pull_stream: impl Stream<Item = Result<Package, anyhow::Error>> + Send,
	pull: P,
) -> Result<(), anyhow::Error> {
	// Pin the streams to use them in loops
	pin_mut!(push_stream);
	pin_mut!(pull_stream);

	// run both pull and push operations until the last one closes or one fails
	let pull = async {
		while let Some(pkg) = pull_stream.next().await {
			pull.pull(pkg?).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	let push = async {
		while let Some(pkg) = push_stream.next().await {
			push.push(pkg?).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	futures::try_join!(pull, push)?;

	Ok(())
}

#[cfg(test)]
pub mod test {

	use super::*;

	pub struct TestSyncer {
		pub sender: tokio::sync::mpsc::Sender<()>,
	}

	impl TestSyncer {
		pub fn new(size: usize) -> (Self, tokio::sync::mpsc::Receiver<()>) {
			let (sender, receiver) = tokio::sync::mpsc::channel(size);
			(Self { sender }, receiver)
		}
	}

	#[async_trait::async_trait]
	impl PushOperations for TestSyncer {
		async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
			println!("push");
			self.sender.send(()).await?;
			Ok(package)
		}
	}

	#[async_trait::async_trait]
	impl PullOperations for TestSyncer {
		async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
			println!("pull");
			self.sender.send(()).await?;
			Ok(package)
		}
	}

	#[tokio::test]
	pub async fn test_example_sync() -> Result<(), anyhow::Error> {
		let (push_syncer, mut push_receiver) = TestSyncer::new(1);
		let (pull_syncer, mut pull_receiver) = TestSyncer::new(10);

		// use a once stream for the push stream
		let push_stream = futures::stream::once(async { Ok(Package::null()) });

		// use a 10 ms interval stream for the pull stream
		let pull_stream = async_stream::stream! {
			for _ in 0..10 {
				tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
				yield Ok(Package::null());
			}
		};

		// let the sync function run for 100 milliseconds
		sync(push_stream, push_syncer, pull_stream, pull_syncer).await?;

		// check that the push operations were called by draining the receiver
		let mut push_receiver_count = 0;
		while let Some(_) = push_receiver.recv().await {
			push_receiver_count += 1;
		}
		assert_eq!(push_receiver_count, 1);

		// check that the pull operations were called by draining the receiver
		let mut pull_receiver_count = 0;
		while let Some(_) = pull_receiver.recv().await {
			pull_receiver_count += 1;
		}
		assert_eq!(pull_receiver_count, 10);

		Ok(())
	}
}
