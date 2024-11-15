use super::BinpackingWeighted;
use crate::grouping_heuristic::{GroupingHeuristic, GroupingOutcome};

/// Implements 1.7 OPT First Fit binpacking heuristic,
/// where OPT is the minimum number of bins required to pack all elements.
/// This means that, if the optimal packing requires 10 bins,
/// the heuristic will require at most 17 bins.
///
/// First Fit binpacking was proved 1.7 OPT by Dosa and Sgall, 2013: doi:10.4230/LIPIcs.STACS.2013.538
///
/// First Fit is particularly suitable for situations where the original order should not be changed.
///
/// This implementation does not allow for elements heavier than the capacity to overflow the bins. So, if you are looking to apply this in a stack with, for example, a splitting heuristic, you should consider wrapping with the CatchError heuristic.
pub struct FirstFitBinpacking {
	pub capacity: usize,
}

impl FirstFitBinpacking {
	pub fn new(capacity: usize) -> Self {
		Self { capacity }
	}

	pub fn boxed(capacity: usize) -> Box<Self> {
		Box::new(Self::new(capacity))
	}
}

impl<T> GroupingHeuristic<T> for FirstFitBinpacking
where
	T: BinpackingWeighted,
{
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		// Flatten all of the elements
		let elements: Vec<_> =
			distribution.into_iter().flat_map(|outcome| outcome.into_inner()).collect();

		// Prepare the result vector
		let mut result: Vec<GroupingOutcome<T>> = Vec::new();

		for element in elements.into_iter() {
			// if the element is heavier than the capacity, return an error
			if element.weight() > self.capacity {
				return Err(anyhow::anyhow!("Element is heavier than the capacity"));
			}

			// Try to place the current element in the last knapsack
			let remaining = if let Some(last_knapsack) = result.last_mut() {
				let current_weight: usize = last_knapsack.0.iter().map(|item| item.weight()).sum();
				if current_weight + element.weight() <= self.capacity {
					last_knapsack.0.push(element);
					None
				} else {
					Some(element)
				}
			} else {
				Some(element)
			};

			// If the element couldn't be placed in the last knapsack, create a new knapsack
			match remaining {
				Some(current_element) => {
					let mut new_knapsack = Vec::new();
					new_knapsack.push(current_element);
					result.push(new_knapsack.into());
				}
				None => (),
			}
		}

		Ok(result)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use crate::grouping_heuristic::ElementalOutcome;

	#[test]
	fn test_first_fit_binpacking() -> Result<(), anyhow::Error> {
		let mut heuristic = FirstFitBinpacking::new(9);
		let distribution = vec![
			GroupingOutcome::new(vec![
				ElementalOutcome::Apply(1),
				ElementalOutcome::Apply(2),
				ElementalOutcome::Apply(3),
				ElementalOutcome::Apply(4),
			]),
			GroupingOutcome::new(vec![
				ElementalOutcome::Apply(5),
				ElementalOutcome::Apply(6),
				ElementalOutcome::Apply(7),
				ElementalOutcome::Apply(8),
			]),
		];

		let distribution = heuristic.distribute(distribution)?;

		let should_be = vec![
			GroupingOutcome::new(vec![
				ElementalOutcome::Apply(1),
				ElementalOutcome::Apply(2),
				ElementalOutcome::Apply(3),
			]),
			GroupingOutcome::new(vec![ElementalOutcome::Apply(4), ElementalOutcome::Apply(5)]),
			GroupingOutcome::new(vec![ElementalOutcome::Apply(6)]),
			GroupingOutcome::new(vec![ElementalOutcome::Apply(7)]),
			GroupingOutcome::new(vec![ElementalOutcome::Apply(8)]),
		];
		assert_eq!(distribution, should_be);

		Ok(())
	}
}
