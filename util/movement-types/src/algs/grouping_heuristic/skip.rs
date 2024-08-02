use crate::algs::grouping_heuristic::{GroupingHeuristic, GroupingOutcome};

pub struct Skip<T>(pub Box<dyn GroupingHeuristic<T>>);

impl<T> Skip<T> {
	pub fn new(heuristic: Box<dyn GroupingHeuristic<T>>) -> Self {
		Skip(heuristic)
	}

	pub fn boxed(heuristic: Box<dyn GroupingHeuristic<T>>) -> Box<Self> {
		Box::new(Skip(heuristic))
	}

	pub fn skip(
		&self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		Ok(distribution)
	}

	pub fn apply(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		self.0.distribute(distribution)
	}
}

impl<T> GroupingHeuristic<T> for Skip<T> {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		self.skip(distribution)
	}
}

pub struct SkipIf<T> {
	pub condition: bool,
	pub skip: Skip<T>,
}

impl<T> SkipIf<T> {
	pub fn new(condition: bool, heuristic: Box<dyn GroupingHeuristic<T>>) -> Self {
		SkipIf { condition, skip: Skip::new(heuristic) }
	}

	pub fn boxed(condition: bool, heuristic: Box<dyn GroupingHeuristic<T>>) -> Box<Self> {
		Box::new(SkipIf::new(condition, heuristic))
	}

	pub fn set_condition(&mut self, condition: bool) {
		self.condition = condition;
	}
}

impl<T> GroupingHeuristic<T> for SkipIf<T> {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		if self.condition {
			self.skip.skip(distribution)
		} else {
			self.skip.apply(distribution)
		}
	}
}

pub struct SkipFor<T> {
	pub skip_if: SkipIf<T>,
	pub counter: usize,
	pub for_count: usize,
}

impl<T> SkipFor<T> {
	pub fn new(count: usize, heuristic: Box<dyn GroupingHeuristic<T>>) -> Self {
		SkipFor { skip_if: SkipIf::new(false, heuristic), counter: 0, for_count: count }
	}

	pub fn boxed(count: usize, heuristic: Box<dyn GroupingHeuristic<T>>) -> Box<Self> {
		Box::new(SkipFor::new(count, heuristic))
	}

	pub fn evaluate(&mut self) {
		self.skip_if.set_condition(self.counter < self.for_count);
	}

	pub fn increment_counter(&mut self) {
		self.counter += 1;
	}
}

impl<T> GroupingHeuristic<T> for SkipFor<T> {
	fn distribute(
		&mut self,
		distribution: Vec<GroupingOutcome<T>>,
	) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
		self.evaluate();
		let distribution = self.skip_if.distribute(distribution);
		self.increment_counter();
		distribution
	}
}
