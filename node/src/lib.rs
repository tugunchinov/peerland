// TODO: remove
#![allow(dead_code)]

const DEFAULT_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(20);

mod communication;
mod error;
mod processing;
mod time;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::NodeError;
use communication::proto;
use network::types::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use sync::SpinLock;
use tokio::sync::Mutex;

pub(crate) struct Node<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, R: Rng> {
    id: u32,

    // TODO: use custom protocol over UDP?
    socket: TcpListener,

    // TODO: make service
    storage: SpinLock<Vec<Vec<u8>>>,

    // TODO: lock-free?
    processed_messages: Mutex<HashSet<uuid::Uuid>>,

    established_connections: SpinLock<HashMap<SocketAddr, Arc<TcpStream>>>,

    system_time_provider: ST,
    logical_time_provider: LT,

    entropy: R,
}

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    pub async fn new(
        id: u32,
        addr: impl ToSocketAddrs,
        time_provider: ST,
        entropy: R,
    ) -> Result<Arc<Self>, NodeError> {
        let socket = TcpListener::bind(addr).await?;

        let logical_time_provider = LT::new_with_id(id);

        let node = Arc::new(Self {
            id,
            socket,
            storage: SpinLock::new(vec![]),
            processed_messages: Mutex::new(HashSet::new()),
            established_connections: SpinLock::new(HashMap::new()),
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
