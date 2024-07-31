pub trait Splitting where Self: Sized {
    fn split(self) -> Vec<Self>;
}