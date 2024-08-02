use crate::grouping_heuristic::{GroupingHeuristic, GroupingOutcome};

pub struct Chunking {
	pub size: usize,
}

impl Chunking {
	pub fn new(size: usize) -> Self {
		Self { size }
	}

	pub fn boxed(size: usize) -> Box<Self> {
		Box::new(Self::new(size))
	}
}

impl<T> GroupingHeuristic<T> for Chunking {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		// flatten the distribution
		let mut distribution = distribution
			.into_iter()
			.flat_map(|outcome| outcome.into_inner())
			.collect::<Vec<_>>();

		// chunk the distribution
		let mut chunks = Vec::new();
		while !distribution.is_empty() {
			let chunk = distribution.drain(0..self.size.min(distribution.len())).collect();
			chunks.push(GroupingOutcome::new(chunk));
		}

		Ok(chunks)
	}
}

pub struct LinearlyDecreasingChunking {
	pub chunking: Chunking,
	pub decreasing_factor: usize,
}

impl LinearlyDecreasingChunking {
	pub fn new(size: usize, decreasing_factor: usize) -> Self {
		Self { chunking: Chunking::new(size), decreasing_factor }
	}

	pub fn boxed(size: usize, decreasing_factor: usize) -> Box<Self> {
		Box::new(Self::new(size, decreasing_factor))
	}
}

impl<T> GroupingHeuristic<T> for LinearlyDecreasingChunking {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		// if the chunk size is 0, return the distribution with all set to terminal status
		if self.chunking.size == 0 {
			return Ok(distribution.into_iter().map(|outcome| outcome.all_to_terminal()).collect());
		}

		// otherwise use the chunking field to chunk the distribution
		let distribution = self.chunking.distribute(distribution)?;

		// decrease the chunk size by 1
		self.chunking.size = self.chunking.size.saturating_sub(self.decreasing_factor);

		Ok(distribution)
	}
}
