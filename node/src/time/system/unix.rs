use crate::time::{Millis, SystemTimeProvider};

pub struct UnixTimeProvider;

impl SystemTimeProvider for UnixTimeProvider {
    fn now_millis(&self) -> Millis {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis()
            .into()
    }
}
