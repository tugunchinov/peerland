use network::types::SocketAddr;

#[derive(thiserror::Error, Debug)]
pub enum NodeError {
    #[error("timed out: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("io error: {0:?}")]
    IOError(#[from] std::io::Error),
    #[error("unknown peer: {0}")]
    UnknownPeer(SocketAddr),
    #[error("decode error: {0}")]
    DecodeError(String),
}

impl From<std::str::Utf8Error> for NodeError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::DecodeError(value.to_string())
    }
}

impl From<std::string::FromUtf8Error> for NodeError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::DecodeError(value.to_string())
    }
}

impl From<prost::DecodeError> for NodeError {
    fn from(value: prost::DecodeError) -> Self {
        Self::DecodeError(value.to_string())
    }
}
