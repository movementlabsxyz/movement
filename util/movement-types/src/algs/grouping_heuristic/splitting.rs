use crate::algs::grouping_heuristic::{
	ElementalFailure, ElementalOutcome, GroupingHeuristic, GroupingOutcome,
};
use itertools::Itertools;

pub trait Splitable
where
	Self: Sized,
{
	fn split(self, factor: usize) -> Vec<Self>;
}

impl<T> Splitable for ElementalFailure<T>
where
	T: Splitable,
{
	fn split(self, factor: usize) -> Vec<Self> {
		match self {
			ElementalFailure::Instrumental(t) => {
				t.split(factor).into_iter().map(ElementalFailure::Instrumental).collect()
			}
			ElementalFailure::Terminal(t) => {
				t.split(factor).into_iter().map(ElementalFailure::Terminal).collect()
			}
		}
	}
}
impl<T> Splitable for ElementalOutcome<T>
where
	T: Splitable,
{
	fn split(self, factor: usize) -> Vec<Self> {
		match self {
			ElementalOutcome::Success => vec![ElementalOutcome::Success],
			ElementalOutcome::Apply(t) => {
				t.split(factor).into_iter().map(ElementalOutcome::Apply).collect()
			}
			ElementalOutcome::Failure(failure) => {
				failure.split(factor).into_iter().map(ElementalOutcome::Failure).collect()
			}
		}
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
		// reform each group by splitting each elemental outcome
		let distribution = distribution
			.into_iter()
			.map(|outcome| {
				outcome
					.into_inner()
					.into_iter()
					.map(|outcome| outcome.split(self.factor))
					.flatten()
					.collect::<Vec<_>>()
					.into()
			})
			.collect::<Vec<_>>();

		Ok(distribution)
	}
}

impl<T> Splitable for Vec<T> {
	fn split(self, factor: usize) -> Vec<Self> {
		let chunk_size = (self.len() as f64 / factor as f64).ceil() as usize;

		// Calculate the chunk size based on the factor
		self.into_iter()
			.chunks(chunk_size)
			.into_iter()
			.map(|chunk| chunk.collect())
			.collect()
	}
}

mod block {

	use super::*;
	use crate::Block;

	impl Splitable for Block {
		fn split(self, factor: usize) -> Vec<Self> {
			// unpack the transactions
			let Block { metadata, transactions, parent, id: _ } = self;

			// split the vector of transactions
			let split_transactions = transactions.split(factor);

			// create a new block for each split transaction
			let mut blocks = Vec::new();
			for split in split_transactions {
				let parent = Block::new(metadata.clone(), parent.clone(), split);
				blocks.push(parent);
			}

			blocks
		}
	}
}
