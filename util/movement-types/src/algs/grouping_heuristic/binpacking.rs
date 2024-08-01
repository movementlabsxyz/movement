use crate::algs::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome,
    ElementalOutcome,
    ElementalFailure
};

pub trait BinpackingWeighted {
    fn weight(&self) -> usize;
}

impl <T> BinpackingWeighted for ElementalFailure<T>
where T: BinpackingWeighted
{

    fn weight(&self) -> usize {
        match self {
            ElementalFailure::Instrumental(t) => t.weight(),
            ElementalFailure::Terminal(t) => t.weight()
        }
    }

}

impl <T> BinpackingWeighted for ElementalOutcome<T>
where T: BinpackingWeighted
{

    fn weight(&self) -> usize {
        match self {
            ElementalOutcome::Success => 0,
            ElementalOutcome::Apply(t) => t.weight(),
            ElementalOutcome::Failure(failure) => failure.weight()
        }
    }

}

/// Implements an 11/9 OPT + 6/9 First Fit Decreasing binpacking heuristic,
/// where OPT is the minimum number of bins required to pack all elements.
/// This means that, if the optimal packing requires 9 bins,
/// the heuristic will require at most 11 bins.
/// The tight-bound of 11/9 OPT + 6/9 instead of 11/9 OPT + 1 was first proven by Dosa 2007: https://link.springer.com/chapter/10.1007/978-3-540-74450-4_1
pub struct FFDBinpacking {
    pub capacity: usize
}

impl FFDBinpacking {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }
}

impl <T> GroupingHeuristic<T> for FFDBinpacking
where T: BinpackingWeighted {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

        // Flatten all of the elements
        let elements: Vec<_> = distribution.into_iter().flat_map(|outcome| outcome.into_inner()).collect();
        
        // Prepare the result vector
        let mut result: Vec<GroupingOutcome<T>> = Vec::new();
        
        // Sort elements by weight (optional, can be optimized for better packing)
        let mut elements = elements;
        elements.sort_by(|a, b| b.weight().cmp(&a.weight()));
        
        while !elements.is_empty() {
            let mut current_knapsack = Vec::new();
            let mut current_weight = 0;
            
            let mut i = 0;
            while i < elements.len() {
                if current_weight + elements[i].weight() <= self.capacity {
                    current_weight += elements[i].weight();
                    current_knapsack.push(elements.remove(i));
                } else {
                    i += 1;
                }
            }
            
            // Add the current knapsack to the result
            result.push(current_knapsack.into());

        }

        Ok(result)
        
    }

}
