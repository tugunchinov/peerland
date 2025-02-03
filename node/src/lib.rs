mod communication;
mod error;
mod proto;
mod time;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::NodeError;
use network::discovery::Discovery;
use network::types::*;
use prost::Message;
use proto::message::NodeMessage;
use rand::Rng;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

    // TODO: better
    // known_nodes: Mutex<Vec<SocketAddr>>,
    discovery: D,

    system_time_provider: ST,
    logical_time_provider: LT,

    entropy: R,
}

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
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
            // known_nodes: Mutex::new(vec![]),
            discovery,
            system_time_provider: time_provider,
            logical_time_provider,
            entropy,
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen_messages().await });
        }

        Ok(node)
    }

    async fn listen_messages(self: Arc<Self>) {
        use crate::proto::message::*;

        tracing::info!("start listening messages (node {})", self.id);
        loop {
            match self.recv_message().await {
                Ok((sender, msg)) => {
                    // TODO:
                    if sender == self.socket.local_addr().unwrap() {
                        tracing::warn!("skip message from myself");
                        continue;
                    }

                    // TODO: extract function

                    let payload = msg.payload;

                    self.storage.lock().await.push(payload.clone());

                    if let Some(msg_kind) = msg.message_kind {
                        match msg_kind {
                            MessageKind::Broadcast(b) => {
                                if let Ok(broadcast_type) = b.try_into() {
                                    match broadcast_type {
                                        broadcast::BroadcastType::Gossip => {
                                            self.gossip(payload, 3).await;
                                        }
                                    }
                                } else {
                                    unreachable!()
                                }
                            }
                            MessageKind::Addressed(a) => {
                                // TODO:
                                tracing::warn!("not implemented");
                            }
                        }
                    } else {
                        tracing::warn!(msg_id = ?msg.id, "unknown message kind");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "failed receiving message");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    async fn send_serialized_message(
        &self,
        serialized_msg: &[u8],
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        // TODO: from config
        const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(10);

        let mut stream = tokio::time::timeout(TIMEOUT, TcpStream::connect(to)).await??;

        tracing::info!(recipient = ?stream.peer_addr(), "sending message");

        tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            stream.write_all(serialized_msg),
        )
        .await??;

        Ok(())
    }

    async fn recv_message(&self) -> Result<(SocketAddr, NodeMessage), NodeError> {
        // TODO: smarter bound
        const MAX_MSG_SIZE_BYTES: usize = 8 * 1024;
        let mut buf = Vec::with_capacity(MAX_MSG_SIZE_BYTES);
        let (mut stream, sender) = self.socket.accept().await?;
        loop {
            let bytes_read = stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
        }

        let deserialized_msg = NodeMessage::decode(buf.as_slice())?;

        self.logical_time_provider
            .adjust_from_message(&deserialized_msg);
        self.logical_time_provider.tick();

        Ok((sender, deserialized_msg))
    }

    fn create_serialized_node_message<B: AsRef<[u8]>>(
        &self,
        data: B,
        msg_kind: proto::message::MessageKind,
    ) -> Vec<u8> {
        use proto::message;
        let ts = self.system_time_provider.now_millis().into();

        let msg = NodeMessage {
            id: Some(message::Uuid {
                value: uuid::Uuid::new_v4().into(),
            }),
            message_kind: Some(msg_kind),
            ts: Some(ts),
            lt: Some(self.logical_time_provider.tick().into()),
            payload: data.as_ref().to_vec(),
        };

        msg.encode_to_vec()
    }

    #[cfg(debug_assertions)]
    async fn get_messages_storage(&self) -> Vec<Vec<u8>> {
        self.storage.lock().await.clone()
    }
}
