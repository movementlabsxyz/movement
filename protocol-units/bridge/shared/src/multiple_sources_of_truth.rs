use futures::task::{Context, Poll};
use futures::{stream::Stream, FutureExt, StreamExt};
use futures_timer::Delay;
use std::{
	cmp::Eq,
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	pin::Pin,
	time::Duration,
};

struct EventInfo {
	sources: HashSet<u64>,
	timestamp: Delay,
}

pub struct MultipleSourceOfTruth<S, E> {
	sources: Vec<S>,
	emitted_events: HashMap<E, EventInfo>,
	threshold: usize,
	processed: HashSet<E>,
	timeout: Duration,
}

impl<S, E> MultipleSourceOfTruth<S, E>
where
	S: Stream<Item = E> + Unpin + Hash,
	E: Eq + Hash + Clone,
{
	pub fn new(sources: Vec<S>, threshold: usize, timeout: Duration) -> Self {
		Self {
			sources,
			emitted_events: HashMap::new(),
			threshold,
			processed: HashSet::new(),
			timeout,
		}
	}
}

pub fn hash_of_source<S: Hash>(source: &S) -> u64 {
	let mut hasher = DefaultHasher::new();
	source.hash(&mut hasher);
	hasher.finish()
}

impl<S, E> Stream for MultipleSourceOfTruth<S, E>
where
	S: Stream<Item = E> + Hash + Unpin,
	E: Eq + Hash + Clone + Unpin,
{
	type Item = E;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		let mut emitted_event = None;

		// Verwijder verlopen events
		this.emitted_events.retain(|event, info| match info.timestamp.poll_unpin(cx) {
			Poll::Ready(()) => {
				this.processed.insert(event.clone());
				false
			}
			Poll::Pending => true,
		});

		for source in &mut this.sources {
			match source.poll_next_unpin(cx) {
				Poll::Ready(Some(event)) => {
					if !this.processed.contains(&event) {
						let info = this.emitted_events.entry(event.clone()).or_insert(EventInfo {
							sources: HashSet::new(),
							timestamp: Delay::new(this.timeout),
						});
						info.sources.insert(hash_of_source(source));
						if info.sources.len() >= this.threshold {
							emitted_event = Some(event.clone());
							this.processed.insert(event.clone());
							break;
						}
					}
				}
				Poll::Ready(None) | Poll::Pending => continue,
			}
		}

		if let Some(event) = emitted_event.clone() {
			this.emitted_events.remove(&event);
			this.processed.insert(event.clone());
			return Poll::Ready(Some(event));
		}
		Poll::Pending
	}
}
