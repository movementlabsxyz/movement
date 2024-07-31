use crate::algs::grouping_heuristic::{
    GroupingHeuristic,
    GroupingOutcome,
    ElementalOutcome,
    ElementalFailure
};

pub trait Splitable where Self: Sized {
    fn split(self, factor : usize) -> Vec<Self>;
}

impl <T> Splitable for ElementalFailure<T>  where T : Splitable {

    fn split(self, factor : usize) -> Vec<Self> {
        
        match self {
            ElementalFailure::Instrumental(t) => t.split(factor).into_iter().map(ElementalFailure::Instrumental).collect(),
            ElementalFailure::Terminal(t) => t.split(factor).into_iter().map(ElementalFailure::Terminal).collect()
        }

    }

}
impl <T> Splitable for ElementalOutcome<T>  where T : Splitable {

    fn split(self, factor : usize) -> Vec<Self> {
        
        match self {
            ElementalOutcome::Success => vec![ElementalOutcome::Success],
            ElementalOutcome::Apply(t) => t.split(factor).into_iter().map(ElementalOutcome::Apply).collect(),
            ElementalOutcome::Failure(failure) => failure.split(factor).into_iter().map(ElementalOutcome::Failure).collect()
        }

    }

}

pub struct Splitting {
    pub factor : usize
}

impl <T> GroupingHeuristic<T> for Splitting 
where T: Splitable {
    
    fn distribute(&mut self, distribution: Vec<GroupingOutcome<T>>) -> Result<Vec<GroupingOutcome<T>>, anyhow::Error> {

        // reform each group by splitting each elemental outcome
        let distribution = distribution.into_iter().map(|outcome| 
            outcome.into_inner().into_iter().map(|outcome| outcome.split(self.factor)).flatten().collect::<Vec<_>>().into()
        ).collect::<Vec<_>>();

        Ok(distribution)
        
    }

}