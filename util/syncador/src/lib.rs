pub mod backend;
pub mod files;
pub mod runner;

pub use crate::backend::{PullOperations, PushOperations};
pub use crate::files::package::{Package, PackageElement};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

pub async fn sync<
	// The pull operations
	Q: PushOperations + Send + Sync + 'static,
	// The push operations
	P: PullOperations + Send + Sync + 'static,
>(
	// The stream that triggers the push operations
	push_stream: Pin<Box<dyn Stream<Item = ()> + Send>>,
	// The push operations
	push: Q,
	// the stream the triggers the pull operations
	pull_stream: Pin<Box<dyn Stream<Item = ()> + Send>>,
	// The pull operations
	pull: P,
) -> Result<(), anyhow::Error> {
	// run both pull and push operations until the last one closes or one fails

	let pull = async {
		let mut pull_stream = pull_stream;
		loop {
			pull_stream.next().await;
			pull.pull(Package::null()).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	let push = async {
		let mut push_stream = push_stream;
		loop {
			push_stream.next().await;
			push.push(Package::null()).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	futures::try_join!(pull, push)?;

	Ok(())
}
