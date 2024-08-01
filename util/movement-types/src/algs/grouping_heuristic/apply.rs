use crate::algs::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome
};

pub struct ToApply;

impl ToApply {
    
    pub fn new() -> Self {
        ToApply
    }

    pub fn boxed() -> Box<Self> {
        Box::new(ToApply)
    }
    
}

impl <T> GroupingHeuristic<T> for ToApply {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

        // convert all outcomes in all of the grouping outcome to apply
        let distribution = distribution.into_iter().map(|outcome| outcome.all_to_apply()).collect::<Vec<_>>();

        Ok(distribution)
        
    }

}