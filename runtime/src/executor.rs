use crate::task::Task;
use futures::future::BoxFuture;
use std::sync::mpsc;

struct Executor {
    queue: mpsc::Receiver<Task<BoxFuture<'static, ()>>>,
}

impl Executor {
    pub fn new() -> Self {
        todo!()
    }

    pub fn run(&self) {
        while let Ok(_task) = self.queue.recv() {
            todo!()
        }
    }
}
