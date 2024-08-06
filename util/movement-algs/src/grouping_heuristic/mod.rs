pub mod apply;
pub mod binpacking;
pub mod chunking;
pub mod drop_success;
pub mod skip;
pub mod splitting;

use std::fmt::Debug;

/// A failure type for a single member of the heuristically formed group.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementalFailure<T> {
	/// An instrumental failure is intended to be be passed on in future iterations.
	Instrumental(T),
	/// A terminal failure is intended to be dropped or pause the execution altogether.
	Terminal(T),
}

impl<T> ElementalFailure<T> {
	/// Returns true if the failure is instrumental.
	pub fn is_instrumental(&self) -> bool {
		match self {
			ElementalFailure::Instrumental(_) => true,
			ElementalFailure::Terminal(_) => false,
		}
	}

	/// Returns true if the failure is terminal.
	pub fn is_terminal(&self) -> bool {
		match self {
			ElementalFailure::Instrumental(_) => false,
			ElementalFailure::Terminal(_) => true,
		}
	}

	/// Converts the failure to a terminal failure.
	/// If a terminal failure is already present, it will return itself.
	pub fn to_terminal(self) -> Self {
		match self {
			ElementalFailure::Instrumental(t) => ElementalFailure::Terminal(t),
			ElementalFailure::Terminal(t) => ElementalFailure::Terminal(t),
		}
	}

	/// Converts the failure to an instrumental failure.
	/// If an instrumental failure is already present, it will return itself.
	pub fn to_instrumental(self) -> Self {
		match self {
			ElementalFailure::Instrumental(t) => ElementalFailure::Instrumental(t),
			ElementalFailure::Terminal(t) => ElementalFailure::Instrumental(t),
		}
	}

	/// Converts the failure to an apply outcome.
	pub fn into_inner(self) -> T {
		match self {
			ElementalFailure::Instrumental(t) => t,
			ElementalFailure::Terminal(t) => t,
		}
	}
}

/// An outcome for a single member of the heuristically formed group.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementalOutcome<T> {
	/// Apply is intended to be used by the inner method in the next iteration.
	Apply(T),
	/// Success is intended to indicate the method completed successfully and no more iteration is needed.
	/// TODO: consider whether this should also wrap T
	Success,
	/// Failure is intended to indicate the method failed. Usually, the element wrapped will either be dropped or regrouped.
	Failure(ElementalFailure<T>),
}

impl<T> ElementalOutcome<T> {
	/// Creates a new apply outcome.
	pub fn new_apply(t: T) -> Self {
		ElementalOutcome::Apply(t)
	}

	/// Returns true if the outcome is a success.
	pub fn is_success(&self) -> bool {
		match self {
			ElementalOutcome::Apply(_) => false,
			ElementalOutcome::Success => true,
			ElementalOutcome::Failure(_) => false,
		}
	}

	/// Returns true if the outcome is a failure.
	pub fn is_failure(&self) -> bool {
		match self {
			ElementalOutcome::Apply(_) => false,
			ElementalOutcome::Success => false,
			ElementalOutcome::Failure(_) => true,
		}
	}

	/// Returns true if the outcome is an apply.
	pub fn is_apply(&self) -> bool {
		match self {
			ElementalOutcome::Apply(_) => true,
			ElementalOutcome::Success => false,
			ElementalOutcome::Failure(_) => false,
		}
	}

	/// Returns true if the outcome is done.
	pub fn is_done(&self) -> bool {
		match self {
			ElementalOutcome::Apply(_) => false,
			ElementalOutcome::Success => true,
			ElementalOutcome::Failure(f) => f.is_terminal(),
		}
	}

	/// Converts the outcome to a terminal failure.
	/// Success is not converted to a terminal failure.
	pub fn to_terminal(self) -> Self {
		match self {
			ElementalOutcome::Apply(t) => ElementalOutcome::Failure(ElementalFailure::Terminal(t)),
			ElementalOutcome::Success => ElementalOutcome::Success,
			ElementalOutcome::Failure(f) => ElementalOutcome::Failure(f.to_terminal()),
		}
	}

	/// Converts the outcome to an instrumental failure.
	/// Success is not converted to an instrumental failure.
	pub fn to_instrumental(self) -> Self {
		match self {
			ElementalOutcome::Apply(t) => {
				ElementalOutcome::Failure(ElementalFailure::Instrumental(t))
			}
			ElementalOutcome::Success => ElementalOutcome::Success,
			ElementalOutcome::Failure(f) => ElementalOutcome::Failure(f.to_instrumental()),
		}
	}

	/// Converts an outcome to a failure, preserving it's instrumental or terminal status.
	pub fn to_failures_prefer_instrumental(self) -> Self {
		match self {
			ElementalOutcome::Apply(t) => {
				ElementalOutcome::Failure(ElementalFailure::Instrumental(t))
			}
			ElementalOutcome::Success => ElementalOutcome::Success,
			ElementalOutcome::Failure(f) => ElementalOutcome::Failure(f),
		}
	}

	/// Converts the outcome to an apply outcome.
	/// Success is not converted to an apply outcome.
	pub fn to_apply(self) -> Self {
		match self {
			ElementalOutcome::Apply(t) => ElementalOutcome::Apply(t),
			ElementalOutcome::Success => ElementalOutcome::Success,
			ElementalOutcome::Failure(f) => ElementalOutcome::Apply(f.into_inner()),
		}
	}
}

/// The outcomes for a particular group in a grouping heuristic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupingOutcome<T>(pub Vec<ElementalOutcome<T>>);

impl<T> GroupingOutcome<T> {
	pub fn new_all_success(size: usize) -> Self {
		let mut outcomes = Vec::new();
		for _ in 0..size {
			outcomes.push(ElementalOutcome::Success);
		}
		Self(outcomes)
	}

	pub fn new_apply(raw_elements: Vec<T>) -> Self {
		Self(raw_elements.into_iter().map(ElementalOutcome::new_apply).collect())
	}

	/// Creates a new grouping outcome with all apply outcomes in the 0th position.
	pub fn new_apply_distribution(raw_elements: Vec<T>) -> Vec<Self> {
		// Place all of the elements into the 0th position in the distribution under one grouping outcome
		let outcome = raw_elements.into();
		vec![outcome]
	}

	pub fn new(outcomes: Vec<ElementalOutcome<T>>) -> Self {
		Self(outcomes)
	}

	pub fn to_failures_prefer_instrumental(self) -> Self {
		Self(
			self.into_inner()
				.into_iter()
				.map(|outcome| outcome.to_failures_prefer_instrumental())
				.collect(),
		)
	}

	/// Returns true if all of the outcomes are successes.
	pub fn all_succeeded(&self) -> bool {
		self.0.iter().all(|outcome| outcome.is_success())
	}

	/// Converts all failures to terminal failures.
	/// This is useful when a grouping heuristic wants to terminate the grouping process.
	pub fn all_to_terminal(self) -> Self {
		Self(self.0.into_iter().map(|outcome| outcome.to_terminal()).collect())
	}

	/// Converts all outcomes to applies.
	pub fn all_to_apply(self) -> Self {
		Self(self.0.into_iter().map(|outcome| outcome.to_apply()).collect())
	}

	/// Checks whether all outcomes are done.
	pub fn all_done(&self) -> bool {
		self.0.iter().all(|outcome| outcome.is_done())
	}

	/// Converts to inner.
	pub fn into_inner(self) -> Vec<ElementalOutcome<T>> {
		self.0
	}

	/// Converts a grouping to a Vec<T>, i.e., a collection of the original type without outcome wrappers.
	/// Drops success elemental outcomes.
	pub fn into_original(self) -> Vec<T> {
		// Collect the outcomes
		self.0
			.into_iter()
			.filter_map(|outcome| match outcome {
				ElementalOutcome::Apply(t) => Some(t),
				ElementalOutcome::Success => None,
				ElementalOutcome::Failure(failure) => match failure {
					ElementalFailure::Instrumental(t) => Some(t),
					ElementalFailure::Terminal(t) => Some(t),
				},
			})
			.collect()
	}
}

impl<T> From<Vec<ElementalOutcome<T>>> for GroupingOutcome<T> {
	fn from(outcome: Vec<ElementalOutcome<T>>) -> Self {
		Self(outcome)
	}
}

impl<T> From<Vec<T>> for GroupingOutcome<T> {
	fn from(outcome: Vec<T>) -> Self {
		Self(outcome.into_iter().map(ElementalOutcome::new_apply).collect())
	}
}

pub trait GroupingHeuristic<T>
where
	T: Sized,
{
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error>;
}

pub struct GroupingHeuristicStack<T>(pub Vec<Box<dyn GroupingHeuristic<T>>>);

impl<T> GroupingHeuristicStack<T> {
	pub fn new(grouping: Vec<Box<dyn GroupingHeuristic<T>>>) -> Self {
		Self(grouping)
	}

	pub fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		let mut distribution = distribution;
		for heuristic in &mut self.0 {
			distribution = heuristic.distribute(distribution)?;
		}
		Ok(distribution)
	}

	/// Runs the grouping heuristic synchronously.
	pub async fn run(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
		func: impl Fn(GroupingOutcome<T>) -> Result<GroupingOutcome<T>, anyhow::Error>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		let mut distribution = distribution;
		loop {
			// distribute
			distribution = self.distribute(distribution)?;

			// run the function
			let mut new_distribution = Vec::new();
			for outcome in distribution {
				new_distribution.push(func(outcome)?);
			}

			// check if we're done
			if new_distribution.iter().all(|outcome| outcome.all_done()) {
				return Ok(new_distribution);
			}

			// update the distribution
			distribution = new_distribution;
		}
	}

	/// Runs the grouping heuristic asynchronously, but in a sequential manner.
	pub async fn run_async_sequential_with_metadata<F, Fut, M>(
		&mut self,
		mut distribution: Vec<GroupingOutcome<T>>,
		func: F,
		mut metadata: M,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error>
	where
		F: Fn(usize, GroupingOutcome<T>, M) -> Fut + Send + Sync,
		Fut: std::future::Future<Output = Result<(GroupingOutcome<T>, M), anyhow::Error>> + Send,
	{
		loop {
			// distribute
			distribution = self.distribute(distribution)?;

			// run the function asynchronously
			let mut new_distribution = Vec::new();

			// include index in iteration and callback
			for (index, outcome) in distribution.into_iter().enumerate() {
				let (new_outcome, new_metadata) = func(index, outcome, metadata).await?;
				metadata = new_metadata;
				new_distribution.push(new_outcome);
			}

			// check if we're done
			if new_distribution.iter().all(|outcome| outcome.all_done()) {
				return Ok(new_distribution);
			}

			// update the distribution
			distribution = new_distribution;
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::chunking::Chunking;
	use super::*;
	use std::sync::Arc;
	use tokio::sync::RwLock;

	#[tokio::test]
	async fn test_async_run_sequential_success() -> Result<(), anyhow::Error> {
		let shared = Arc::new(RwLock::new(0));
		let mut stack = GroupingHeuristicStack::new(vec![Chunking::boxed(2)]);

		let distribution: Vec<GroupingOutcome<usize>> = vec![GroupingOutcome::new_all_success(4)];

		let result = stack
			.run_async_sequential_with_metadata(
				distribution,
				|_index, outcome, _metadata| async {
					let mut shared = shared.write().await;
					*shared += 1;
					Ok((outcome, Some(1)))
				},
				Some(1),
			)
			.await?;

		assert_eq!(*shared.read().await, 2);
		assert_eq!(result.len(), 2);
		assert!(result.iter().all(|outcome| outcome.all_succeeded()));

		Ok(())
	}
}
