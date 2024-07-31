pub mod splitting;
pub mod chunking;
pub mod apply;
pub mod drop_success;

pub enum ElementalFailure<T> {
    Instrumental(T),
    Terminal(T)
}

impl <T> ElementalFailure<T> {
    pub fn is_instrumental(&self) -> bool {
        match self {
            ElementalFailure::Instrumental(_) => true,
            ElementalFailure::Terminal(_) => false
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self {
            ElementalFailure::Instrumental(_) => false,
            ElementalFailure::Terminal(_) => true
        }
    }

    pub fn to_terminal(self) -> Self {
        match self {
            ElementalFailure::Instrumental(t) => ElementalFailure::Terminal(t),
            ElementalFailure::Terminal(t) => ElementalFailure::Terminal(t)
        }
    }

    pub fn to_instrumental(self) -> Self {
        match self {
            ElementalFailure::Instrumental(t) => ElementalFailure::Instrumental(t),
            ElementalFailure::Terminal(t) => ElementalFailure::Instrumental(t)
        }
    }

    pub fn into_inner(self) -> T {
        match self {
            ElementalFailure::Instrumental(t) => t,
            ElementalFailure::Terminal(t) => t
        }
    }

}

pub enum ElementalOutcome<T> {
    Apply(T),
    Success,
    Failure(ElementalFailure<T>)
}

impl <T> ElementalOutcome<T> {
    pub fn is_success(&self) -> bool {
        match self {
            ElementalOutcome::Apply(_) => false,
            ElementalOutcome::Success => true,
            ElementalOutcome::Failure(_) => false
        }
    }

    pub fn is_failure(&self) -> bool {
        match self {
            ElementalOutcome::Apply(_) => false,
            ElementalOutcome::Success => false,
            ElementalOutcome::Failure(_) => true
        }
    }

    pub fn is_apply(&self) -> bool {
        match self {
            ElementalOutcome::Apply(_) => true,
            ElementalOutcome::Success => false,
            ElementalOutcome::Failure(_) => false
        }
    }

    pub fn is_done(&self) -> bool {
        match self {
            ElementalOutcome::Apply(_) => false,
            ElementalOutcome::Success => true,
            ElementalOutcome::Failure(f) => f.is_terminal()
        }
    }

    pub fn to_terminal(self) -> Self {
        match self {
            ElementalOutcome::Apply(t) => ElementalOutcome::Failure(ElementalFailure::Terminal(t)),
            ElementalOutcome::Success => ElementalOutcome::Success,
            ElementalOutcome::Failure(f) => ElementalOutcome::Failure(f.to_terminal())
        }
    }

    pub fn to_instrumental(self) -> Self {
        match self {
            ElementalOutcome::Apply(t) => ElementalOutcome::Failure(ElementalFailure::Instrumental(t)),
            ElementalOutcome::Success => ElementalOutcome::Success,
            ElementalOutcome::Failure(f) => ElementalOutcome::Failure(f.to_instrumental())
        }
    }

    pub fn to_apply(self) -> Self {
        match self {
            ElementalOutcome::Apply(t) => ElementalOutcome::Apply(t),
            ElementalOutcome::Success => ElementalOutcome::Success,
            ElementalOutcome::Failure(f) => ElementalOutcome::Apply(f.into_inner())
        }
    }

}

pub struct GroupingOutcome<T>(pub Vec<ElementalOutcome<T>>);

impl <T> GroupingOutcome<T> {
    pub fn new(outcome: Vec<ElementalOutcome<T>>) -> Self {
        Self {
            0: outcome
        }
    }

    pub fn all_succeeded(&self) -> bool {
        self.0.iter().all(|outcome| outcome.is_success())
    }

    /// Converts all failures to terminal failures.
    /// This is useful when a grouping heuristic wants to terminate the grouping process.
    pub fn all_to_terminal(self) -> Self {
        Self {
            0: self.0.into_iter().map(|outcome| outcome.to_terminal()).collect()
        }
    }

    pub fn all_to_apply(self) -> Self {
        Self {
            0: self.0.into_iter().map(|outcome| outcome.to_apply()).collect()
        }
    }

    pub fn all_done(&self) -> bool {
        self.0.iter().all(|outcome| outcome.is_done())
    }



    pub fn into_inner(self) -> Vec<ElementalOutcome<T>> {
        self.0
    }

}

impl <T> From<Vec<ElementalOutcome<T>>> for GroupingOutcome<T> {
    fn from(outcome: Vec<ElementalOutcome<T>>) -> Self {
        Self {
            0: outcome
        }
    }
}

pub trait GroupingHeuristic<T>
    where T: Sized {

    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error>;

}

pub struct GroupingLayers<T>(pub Vec<Box<dyn GroupingHeuristic<T>>>);

impl <T> GroupingLayers<T> {

    pub fn new(grouping: Vec<Box<dyn GroupingHeuristic<T>>>) -> Self {
        Self {
            0: grouping
        }
    }

    pub fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
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
        func: impl Fn(GroupingOutcome<T>) -> Result<GroupingOutcome<T>, anyhow::Error> + Send + Sync
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
    pub async fn run_async_sequential(
        &mut self, 
        mut distribution: Vec<GroupingOutcome<T>>,
        func: impl Fn(GroupingOutcome<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<GroupingOutcome<T>, anyhow::Error>> + Send>> + Send + Sync
    ) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {
        loop {
            // distribute
            distribution = self.distribute(distribution)?;
    
            // run the function asynchronously
            let mut new_distribution = Vec::new();
            for outcome in distribution {
                new_distribution.push(func(outcome).await?);
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