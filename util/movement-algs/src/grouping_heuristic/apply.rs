use crate::grouping_heuristic::{GroupingHeuristic, GroupingOutcome};

pub struct ToApply;

impl ToApply {
	pub fn new() -> Self {
		ToApply
	}

	pub fn boxed() -> Box<Self> {
		Box::new(ToApply)
	}
}

impl<T> GroupingHeuristic<T> for ToApply {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		// convert all outcomes in all of the grouping outcome to apply
		let distribution = distribution
			.into_iter()
			.map(|outcome| outcome.all_to_apply())
			.collect::<Vec<_>>();

		Ok(distribution)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use crate::grouping_heuristic::{ElementalFailure, ElementalOutcome};

	#[test]
	fn test_to_apply() -> Result<(), anyhow::Error> {
		let mut heuristic = ToApply::new();
		let distribution = vec![
			GroupingOutcome::new(vec![
				ElementalOutcome::Success,
				ElementalOutcome::Failure(ElementalFailure::Instrumental(1)),
				ElementalOutcome::Apply(2),
			]),
			GroupingOutcome::new(vec![
				ElementalOutcome::Success,
				ElementalOutcome::Failure(ElementalFailure::Terminal(3)),
				ElementalOutcome::Apply(4),
			]),
		];

		let distribution = heuristic.distribute(distribution)?;

		let should_be = vec![
			GroupingOutcome::new(vec![
				ElementalOutcome::Success,
				ElementalOutcome::Apply(1),
				ElementalOutcome::Apply(2),
			]),
			GroupingOutcome::new(vec![
				ElementalOutcome::Success,
				ElementalOutcome::Apply(3),
				ElementalOutcome::Apply(4),
			]),
		];
		assert_eq!(distribution, should_be);

		Ok(())
	}
}
