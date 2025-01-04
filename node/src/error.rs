#[derive(thiserror::Error, Debug)]
pub enum NodeError {
    #[error("error")]
    Error,
}

impl From<std::io::Error> for NodeError {
    fn from(_value: std::io::Error) -> Self {
        todo!()
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
