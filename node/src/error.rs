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

impl From<rmp_serde::encode::Error> for NodeError {
    fn from(_value: rmp_serde::encode::Error) -> Self {
        todo!()
    }
}

impl From<rmp_serde::decode::Error> for NodeError {
    fn from(_value: rmp_serde::decode::Error) -> Self {
        todo!()
    }
}
