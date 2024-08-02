use crate::grouping_heuristic::{GroupingHeuristic, GroupingOutcome};

/// Heuristic combinator that converts all outcomes to failures if the inner heuristic fails.
/// This relies on cloning the inner heuristic, so it is advised that this be used with types that can be cloned cheaply.
pub struct CatchAllToFailure<T>(pub Box<dyn GroupingHeuristic<T>>)
where
	T: Clone;

impl<T> CatchAllToFailure<T>
where
	T: Clone,
{
	pub fn new(heuristic: Box<dyn GroupingHeuristic<T>>) -> Self {
		Self(heuristic)
	}

	pub fn boxed(heuristic: Box<dyn GroupingHeuristic<T>>) -> Box<Self> {
		Box::new(Self(heuristic))
	}
}

impl<T> GroupingHeuristic<T> for CatchAllToFailure<T>
where
	T: Clone,
{
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		let recoverable = distribution.clone();

		match self.0.distribute(distribution) {
			Ok(outcome) => Ok(outcome),
			Err(_) => {
				let failures = recoverable
					.into_iter()
					.map(|outcomes| outcomes.to_failures_prefer_instrumental())
					.collect();
				Ok(failures)
			}
		}
	}
}
