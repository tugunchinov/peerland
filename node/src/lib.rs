mod error;
mod proto;
mod sync;
mod time;

#[cfg(test)]
mod tests;

use crate::error::NodeError;
use crate::time::LamportClock;
use network::types::*;
use prost::Message;
use proto::message::NodeMessage;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub(crate) struct Node<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider> {
    id: u32,

    // TODO: use custom protocol over UDP?
    socket: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<NodeMessage>>,

    // TODO: better
    // known_nodes: Mutex<Vec<SocketAddr>>,
    system_time_provider: ST,
    // TODO: use trait
    logical_time_provider: LT,
}

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider> Node<ST, LT> {
    pub async fn new(
        id: u32,
        addr: impl ToSocketAddrs,
        time_provider: ST,
    ) -> Result<Arc<Self>, NodeError> {
        let socket = TcpListener::bind(addr).await?;

        let logical_time_provider = LT::new_with_id(id);

        let node = Arc::new(Self {
            id,
            socket,
            storage: Mutex::new(vec![]),
            // known_nodes: Mutex::new(vec![]),
            system_time_provider: time_provider,
            logical_time_provider,
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen_messages().await });
        }

        Ok(node)
    }

    async fn listen_messages(self: Arc<Self>) {
        tracing::info!("start listening messages (node {})", self.id);
        loop {
            match self.recv_message().await {
                Ok(msg) => {
                    self.logical_time_provider.adjust_from_message(&msg);
                    self.logical_time_provider.tick();
                    self.storage.lock().await.push(msg);
                }
                Err(e) => {
                    tracing::error!(error = ?e, "failed receiving message");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub(crate) async fn send_message<B: AsRef<[u8]>>(
        &self,
        msg: B,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        // TODO: from config
        const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(10);

        let serialized_msg = self.create_serialized_node_message(msg);

        let mut stream = tokio::time::timeout(TIMEOUT, TcpStream::connect(to)).await??;
        tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            stream.write_all(&serialized_msg),
        )
        .await??;

        Ok(())
    }

    async fn recv_message(&self) -> Result<NodeMessage, NodeError> {
        // TODO: smarter bound
        const MAX_MSG_SIZE_BYTES: usize = 8 * 1024;
        let mut buf = Vec::with_capacity(MAX_MSG_SIZE_BYTES);
        let (mut stream, _addr) = self.socket.accept().await?;
        loop {
            let bytes_read = stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
        }

        let deserialized_msg = NodeMessage::decode(buf.as_slice())?;
        Ok(deserialized_msg)
    }

    fn create_serialized_node_message<B: AsRef<[u8]>>(&self, data: B) -> Vec<u8> {
        let ts = self.system_time_provider.now_millis().into();

        let msg = NodeMessage {
            id: Some(proto::message::Uuid {
                value: uuid::Uuid::new_v4().into(),
            }),
            kind: proto::message::node_message::MessageKind::Ordinary as i32,
            ts: Some(ts),
            lt: Some(self.logical_time_provider.tick().into()),
            data: data.as_ref().to_vec(),
        };
        msg.encode_to_vec()
    }
}

// TODO: remove (used to fool cargo clippy)
pub async fn dummy() {
    let time_provider = time::UnixTimeProvider;

    let node = Node::<_, LamportClock>::new(
        0,
        (
            std::net::IpAddr::from(std::net::Ipv4Addr::UNSPECIFIED),
            9000,
        ),
        time_provider,
    )
    .await
    .unwrap();

    node.send_message(
        "hello",
        (
            std::net::IpAddr::from(std::net::Ipv4Addr::UNSPECIFIED),
            9000,
        ),
    )
    .await
    .unwrap();
}
