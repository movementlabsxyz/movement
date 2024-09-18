use crate::grouping_heuristic::{
	ElementalFailure, ElementalOutcome, GroupingHeuristic, GroupingOutcome,
};
use itertools::Itertools;

/// A heuristic that splits elements in the distribution into smaller chunks
pub trait Splitable
where
	Self: Sized,
{
	fn split(self, factor: usize) -> Result<Vec<Self>, anyhow::Error>;
}

impl<T> Splitable for ElementalFailure<T>
where
	T: Splitable,
{
	fn split(self, factor: usize) -> Result<Vec<Self>, anyhow::Error> {
		Ok(match self {
			ElementalFailure::Instrumental(t) => {
				t.split(factor)?.into_iter().map(ElementalFailure::Instrumental).collect()
			}
			ElementalFailure::Terminal(t) => {
				t.split(factor)?.into_iter().map(ElementalFailure::Terminal).collect()
			}
		})
	}
}
impl<T> Splitable for ElementalOutcome<T>
where
	T: Splitable,
{
	fn split(self, factor: usize) -> Result<Vec<Self>, anyhow::Error> {
		Ok(match self {
			ElementalOutcome::Success => vec![ElementalOutcome::Success],
			ElementalOutcome::Apply(t) => {
				t.split(factor)?.into_iter().map(ElementalOutcome::Apply).collect()
			}
			ElementalOutcome::Failure(failure) => {
				failure.split(factor)?.into_iter().map(ElementalOutcome::Failure).collect()
			}
		})
	}
}

pub struct Splitting {
	pub factor: usize,
}

impl Splitting {
	pub fn new(factor: usize) -> Self {
		Self { factor }
	}

	pub fn boxed(factor: usize) -> Box<Self> {
		Box::new(Self::new(factor))
	}
}

impl<T> GroupingHeuristic<T> for Splitting
where
	T: Splitable,
{
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		let distribution: Result<Vec<GroupingOutcome<T>>, anyhow::Error> = distribution
			.into_iter()
			.map(|outcome| {
				outcome
					.into_inner()
					.into_iter()
					.map(|inner_outcome| inner_outcome.split(self.factor)) // Now returns Result<Vec<_>, anyhow::Error>
					.collect::<Result<Vec<_>, _>>() // Collects Results and returns a Result<Vec<Vec<_>>, anyhow::Error>
					.map(|vec_of_vecs| vec_of_vecs.into_iter().flatten().collect::<Vec<_>>()) // Flatten the inner Vecs
					.map(GroupingOutcome::new) // Convert the flattened Vec into GroupingOutcome
			})
			.collect::<Result<Vec<_>, _>>(); // Collect the outer Results into a single Result<Vec<GroupingOutcome<T>>, anyhow::Error>

		let distribution = distribution?; // Unwrap the final Result, propagating any errors

		Ok(distribution)
	}
}

impl<T> Splitable for Vec<T> {
	fn split(self, factor: usize) -> Result<Vec<Self>, anyhow::Error> {
		let chunk_size = (self.len() as f64 / factor as f64).ceil() as usize;

		// Calculate the chunk size based on the factor
		let result = self
			.into_iter()
			.chunks(chunk_size)
			.into_iter()
			.map(|chunk| chunk.collect())
			.collect();

		Ok(result)
	}
}

mod block {

	use std::collections::BTreeSet;

	use super::*;
	use movement_types::block::Block;

	impl Splitable for Block {
		fn split(self, factor: usize) -> Result<Vec<Self>, anyhow::Error> {
			// unpack the transactions
			let (metadata, parent, transactions, _id) = self.into_parts();

			// split the vector of transactions
			let split_transactions = Vec::from_iter(transactions).split(factor)?;

			// create a new block for each split transaction
			let mut blocks = Vec::new();
			for split in split_transactions {
				let parent =
					Block::new(metadata.clone(), parent.clone(), BTreeSet::from_iter(split));
				blocks.push(parent);
			}

			Ok(blocks)
		}
	}
}
