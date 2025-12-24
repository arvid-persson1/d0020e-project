#[derive(Debug)]
pub enum Translation<T> {
    Success(T),
}

pub trait Translate<Q> {
    type Output;

    fn translate(query: &Q) -> Translation<Self::Output>;
}
