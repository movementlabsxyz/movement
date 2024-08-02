use crate::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome
};

pub struct DropSuccess;

impl DropSuccess {
    
    pub fn new() -> Self {
        DropSuccess
    }

    pub fn boxed() -> Box<Self> {
        Box::new(DropSuccess)
    }
    
}

impl <T> GroupingHeuristic<T> for DropSuccess {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

        // remove all of the success outcomes
        let distribution = distribution.into_iter().filter(|outcome| 
            outcome.0.iter().any(|outcome| !outcome.is_success())
        ).collect::<Vec<_>>();

        Ok(distribution)
       
    }

}