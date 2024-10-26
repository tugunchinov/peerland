#[derive(thiserror::Error, Debug)]
pub enum NodeError {
    #[error("error")]
    Error,
}
