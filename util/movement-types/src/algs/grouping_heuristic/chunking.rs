use crate::algs::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome
};

pub struct Chunking {
    pub size: usize
}

impl <T> GroupingHeuristic<T> for Chunking {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

        // flatten the distribution
        let mut distribution = distribution.into_iter().flat_map(|outcome| outcome.into_inner()).collect::<Vec<_>>();

        // chunk the distribution
        let mut chunks = Vec::new();
        while !distribution.is_empty() {
            let chunk = distribution.drain(0..self.size.min(distribution.len())).collect();
            chunks.push(GroupingOutcome::new(chunk));
        }

        Ok(chunks)
        
    }

}