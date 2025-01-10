pub enum SerializationError {
    Utf8Error(std::str::Utf8Error),
}

#[derive(thiserror::Error, Debug)]
pub enum NodeError {
    #[error("...")]
    SerializationError {},
    #[error("timed out: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("io error: {0:?}")]
    IOError(std::io::Error),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for NodeError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<std::str::Utf8Error> for NodeError {
    fn from(_value: std::str::Utf8Error) -> Self {
        todo!()
    }
}

impl From<std::string::FromUtf8Error> for NodeError {
    fn from(_value: std::string::FromUtf8Error) -> Self {
        todo!()
    }
}

impl From<prost::DecodeError> for NodeError {
    fn from(_value: prost::DecodeError) -> Self {
        todo!()
    }
}
