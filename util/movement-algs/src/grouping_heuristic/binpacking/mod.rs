pub mod first_fit_decreasing;
pub use first_fit_decreasing::*;
pub mod first_fit;
pub use first_fit::*;

use crate::grouping_heuristic::{ElementalFailure, ElementalOutcome};

pub trait BinpackingWeighted {
	fn weight(&self) -> usize;
}

impl<T> BinpackingWeighted for ElementalFailure<T>
where
	T: BinpackingWeighted,
{
	fn weight(&self) -> usize {
		match self {
			ElementalFailure::Instrumental(t) => t.weight(),
			ElementalFailure::Terminal(t) => t.weight(),
		}
	}
}

impl<T> BinpackingWeighted for ElementalOutcome<T>
where
	T: BinpackingWeighted,
{
	fn weight(&self) -> usize {
		match self {
			ElementalOutcome::Success => 0,
			ElementalOutcome::Apply(t) => t.weight(),
			ElementalOutcome::Failure(failure) => failure.weight(),
		}
	}
}

pub mod numeric {

	use super::*;

	impl BinpackingWeighted for usize {
		fn weight(&self) -> usize {
			*self
		}
	}

	impl BinpackingWeighted for i32 {
		fn weight(&self) -> usize {
			*self as usize
		}
	}

	impl BinpackingWeighted for i64 {
		fn weight(&self) -> usize {
			*self as usize
		}
	}

	impl BinpackingWeighted for f32 {
		fn weight(&self) -> usize {
			*self as usize
		}
	}

	impl BinpackingWeighted for f64 {
		fn weight(&self) -> usize {
			*self as usize
		}
	}
}

mod block {

	use super::*;
	use movement_types::{Block, Id, Transaction};

	impl BinpackingWeighted for Id {
		fn weight(&self) -> usize {
			self.0.len()
		}
	}

	impl BinpackingWeighted for Transaction {
		fn weight(&self) -> usize {
			self.data.len() + self.id.weight()
		}
	}

	impl BinpackingWeighted for Block {
		fn weight(&self) -> usize {
			// sum of the transactions
			let mut weight =
				self.transactions.iter().map(|transaction| transaction.weight()).sum::<usize>();

			// id
			weight += self.id.weight();

			// parent
			weight += self.parent.len();

			// for now metadata is negligible

			weight
		}
	}
}

mod shared {

	use super::*;
	use std::sync::Arc;

	impl<T> BinpackingWeighted for Arc<T>
	where
		T: BinpackingWeighted,
	{
		fn weight(&self) -> usize {
			self.as_ref().weight()
		}
	}
}
