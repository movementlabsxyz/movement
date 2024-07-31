pub trait Splitting<T>
    where T: Sized {
    fn split(self) -> Vec<T>;
}