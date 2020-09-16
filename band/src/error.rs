#[derive(Debug)]
pub enum Error {
    TaskNotFound(String),
    InvalidDepency(String),
}
