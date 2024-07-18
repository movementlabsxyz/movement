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

#[derive(Debug)]
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
	E: Eq + Hash + Clone + Unpin + std::fmt::Debug,
{
	type Item = E;

	#[tracing::instrument(skip_all)]
	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		tracing::trace!("poll_next called");
		let this = self.get_mut();
		tracing::trace!("Current state: {:?}", this.emitted_events);
		let mut emitted_event = None;

		// Remove expired events
		this.emitted_events.retain(|event, info| {
			tracing::trace!("Checking event: {:?}", event);
			match info.timestamp.poll_unpin(cx) {
				Poll::Ready(()) => {
					tracing::trace!("Event expired: {:?}", event);
					this.processed.insert(event.clone());
					false
				}
				Poll::Pending => true,
			}
		});

		for (i, source) in this.sources.iter_mut().enumerate() {
			let source_span = tracing::trace_span!("source",  i=i, hash=?hash_of_source(source));
			let _enter_source = source_span.enter();
			tracing::trace!("polling source");
			match source.poll_next_unpin(cx) {
				Poll::Ready(Some(event)) => {
					let event_span = tracing::trace_span!("event", event=?event);
					let _enter_event = event_span.enter();
					tracing::trace!("received event");
					if !this.processed.contains(&event) {
						tracing::trace!("already processed");
						continue;
					}

					let info = this.emitted_events.entry(event.clone()).or_insert_with(|| {
						tracing::trace!("new event, initiatlizing event info");
						EventInfo { sources: HashSet::new(), timestamp: Delay::new(this.timeout) }
					});
					info.sources.insert(hash_of_source(source));
					tracing::trace!("event info: {:?}", info);
					if info.sources.len() >= this.threshold {
						tracing::trace!("threshold reached for event, emitting");
						emitted_event = Some(event.clone());
						this.processed.insert(event.clone());
						break;
					}
				}
				Poll::Ready(None) | Poll::Pending => continue,
			}
		}

		if let Some(event) = emitted_event {
			tracing::trace!("Emitting event: {:?}", event);
			this.emitted_events.remove(&event);
			this.processed.insert(event.clone());
			return Poll::Ready(Some(event));
		}
		tracing::trace!("No event emitted, returning Poll::Pending");
		Poll::Pending
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	use futures::stream::Stream;
	use futures::task::{Context, Poll};
	use std::collections::VecDeque;
	use std::pin::Pin;

	struct TestSource {
		name: String,
		events: VecDeque<(usize, usize)>,
		event: Option<usize>,
		delay: Option<Delay>,
	}

	impl TestSource {
		fn new(name: &str, events: Vec<(usize, usize)>) -> Self {
			Self {
				name: name.to_string(),
				events: VecDeque::from(events),
				event: None,
				delay: None,
			}
		}
	}

	impl Stream for TestSource {
		type Item = usize;

		#[tracing::instrument(skip_all, fields(name = self.name.as_str()))]
		fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
			let this = self.get_mut();

			loop {
				tracing::trace!("poll_next loop");
				if let Some(delay) = &mut this.delay {
					match delay.poll_unpin(_cx) {
						Poll::Ready(()) => {
							tracing::trace!("emitting event: {:?}", this.event);
							this.delay = None;
							return Poll::Ready(this.event.take());
						}
						Poll::Pending => return Poll::Pending,
					}
				}

				if let Some((event, delay)) = this.events.pop_front() {
					tracing::trace!("Setting event: {} with delay: {}", event, delay);
					this.delay = Some(Delay::new(Duration::from_millis(delay as u64)));
					this.event = Some(event);
					continue;
				} else {
					return Poll::Ready(None);
				}
			}
		}
	}

	impl std::hash::Hash for TestSource {
		fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
			self.name.hash(state);
		}
	}

	#[test_log::test(tokio::test)]
	async fn test_multiple_sources_of_truth() {
		tracing::trace!("test_multiple_sources_of_truth");
		let source1 = TestSource::new("source1", vec![(1, 100), (2, 100), (3, 100)]);
		let source2 = TestSource::new("source2", vec![(1, 150), (2, 150), (3, 150)]);
		let source3 = TestSource::new("source3", vec![(1, 200), (2, 50), (3, 300)]);
		let sources = vec![source1, source2, source3];
		let mut msot = MultipleSourceOfTruth::new(sources, 2, Duration::from_secs(5));

		let mut events = Vec::new();
		while let Some(event) = msot.next().await {
			tracing::trace!("event: {:?}", event);
			events.push(event);
		}

		assert_eq!(events, vec![1, 2, 3]);
	}
}
