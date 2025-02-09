// TODO: remove
#![allow(dead_code)]

mod communication;
mod error;
mod processing;
mod time;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::NodeError;
use communication::proto;
use network::discovery::Discovery;
use network::types::*;
use rand::Rng;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) struct Node<
    ST: time::SystemTimeProvider,
    LT: time::LogicalTimeProvider,
    D: Discovery,
    R: Rng,
> {
    id: u32,

    // TODO: use custom protocol over UDP?
    socket: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<Vec<u8>>>,

    // TODO: lock-free?
    processed_messages: Mutex<HashSet<uuid::Uuid>>,

    discovery: D,

    system_time_provider: ST,
    logical_time_provider: LT,

    entropy: R,
}

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        D: Discovery + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, D, R>
{
    pub async fn new(
        id: u32,
        addr: impl ToSocketAddrs,
        time_provider: ST,
        discovery: D,
        entropy: R,
    ) -> Result<Arc<Self>, NodeError> {
        let socket = TcpListener::bind(addr).await?;

        let logical_time_provider = LT::new_with_id(id);

        let node = Arc::new(Self {
            id,
            socket,
            storage: Mutex::new(vec![]),
            processed_messages: Default::default(),
            discovery,
            system_time_provider: time_provider,
            logical_time_provider,
            entropy,
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen().await });
        }

        Ok(node)
    }
}
