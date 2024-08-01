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

/// Implements 1.7 OPT First Fit binpacking heuristic,
/// where OPT is the minimum number of bins required to pack all elements.
/// This means that, if the optimal packing requires 10 bins,
/// the heuristic will require at most 17 bins.
/// 
/// First Fit binpacking was proved 1.7 OPT by Dosa and Sgall, 2013: doi:10.4230/LIPIcs.STACS.2013.538
/// 
/// First Fit is particularly suitable for situations where the original order should not be changed.
pub struct FirstFitBinpacking {
    pub capacity: usize
}

impl FirstFitBinpacking {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }
}

impl <T> GroupingHeuristic<T> for FirstFitBinpacking
where T: BinpackingWeighted {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

       // Flatten all of the elements
       let elements: Vec<_> = distribution.into_iter().flat_map(|outcome| outcome.into_inner()).collect();
        
       // Prepare the result vector
       let mut result: Vec<GroupingOutcome<T>> = Vec::new();
 
        for mut element in elements.into_iter().map(Some) {
            if let Some(current_element) = element.take() {
       
                // Try to place the current element in the last knapsack
                let remaining = if let Some(last_knapsack) = result.last_mut() {
                    let current_weight: usize = last_knapsack.0.iter().map(|item| item.weight()).sum();
                    if current_weight + current_element.weight() <= self.capacity {
                        last_knapsack.0.push(current_element);
                        None
                    } else {
                        element
                    }
                } else {
                    element
                };

                // If the element couldn't be placed in the last knapsack, create a new knapsack
                match remaining {
                   Some(current_element) =>{
                        let mut new_knapsack = Vec::new();
                        new_knapsack.push(current_element);
                        result.push(new_knapsack.into());
                   },
                   None => ()
                }
            }
        }
       
       Ok(result)
        
    }

}