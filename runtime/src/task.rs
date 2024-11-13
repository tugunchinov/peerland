use std::future::Future;

pub struct Task<F: Future> {
    pub fut: F,
}
