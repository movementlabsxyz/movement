use crate::algs::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome,
};
use super::BinpackingWeighted;

/// Implements an 11/9 OPT + 6/9 First Fit Decreasing binpacking heuristic,
/// where OPT is the minimum number of bins required to pack all elements.
/// This means that, if the optimal packing requires 9 bins,
/// the heuristic will require at most 11 bins.
/// 
/// The tight-bound of 11/9 OPT + 6/9 instead of 11/9 OPT + 1 was first proven by Dosa 2007: doi:10.1007/978-3-540-74450-4_1
/// 
/// First Fit Decreasing is particularly suitable for throughput optimization,
/// where groups must be sent sequentially,
/// as it prioritizes including larger elements in earlier bins.
/// 
/// First Fit Decreasing will not preserve the original order of the elements.
pub struct FirstFitDecreasingBinpacking {
    pub capacity: usize
}

impl FirstFitDecreasingBinpacking {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    pub fn boxed(capacity: usize) -> Box<Self> {
        Box::new(Self::new(capacity))
    }

}

impl <T> GroupingHeuristic<T> for FirstFitDecreasingBinpacking
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

                let weight = elements[i].weight();

                // if the element is heavier than the capacity, return an error
                if weight > self.capacity {
                    return Err(anyhow::anyhow!("Element is heavier than the capacity"));
                }

                if current_weight + weight <= self.capacity {
                    current_weight += weight;
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