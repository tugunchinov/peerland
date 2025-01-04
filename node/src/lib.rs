mod error;
mod proto;

use crate::error::NodeError;
use network::types::*;
use prost::Message;
use proto::message::{node_message, NodeMessage};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub struct Node {
    name: String,

    socket: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<NodeMessage>>,

    // TODO: better
    known_nodes: Mutex<Vec<SocketAddr>>,
}

impl Node {
    pub async fn new<S: AsRef<str>>(
        addr: impl ToSocketAddrs,
        name: S,
    ) -> Result<Arc<Self>, NodeError> {
        let socket = TcpListener::bind(addr).await?;

        let node = Arc::new(Self {
            name: name.as_ref().to_string(),
            socket,
            storage: Mutex::new(vec![]),
            known_nodes: Mutex::new(vec![]),
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen_messages().await });
        }

        Ok(node)
    }

    async fn listen_messages(self: Arc<Self>) {
        tracing::info!("start listening messages");
        loop {
            match self.recv_message().await {
                Ok(msg) => {
                    tracing::info!(message = ?msg, "received message");
                    self.storage.lock().await.push(msg);
                }
                Err(e) => {
                    tracing::error!(error = ?e, "failed receiving message");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub(crate) async fn send_message(
        &self,
        msg: &NodeMessage,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        let serialized_msg = msg.encode_to_vec();
        let mut stream = TcpStream::connect(to).await?;
        stream.write_all(&serialized_msg).await?;
        Ok(())
    }

    async fn recv_message(&self) -> Result<NodeMessage, NodeError> {
        const MAX_MSG_SIZE_BYTES: usize = 8 * 1024;

        let mut buf = Vec::with_capacity(MAX_MSG_SIZE_BYTES);
        let (mut stream, addr) = self.socket.accept().await?;
        tracing::info!(node = ?self.name, from = ?addr, "received new message");

        loop {
            let bytes_read = stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
            tracing::trace!(bytes_read = ?bytes_read, buffer_len = ?buf.len());
        }

        let deserialized_msg = NodeMessage::decode(buf.as_slice())?;

        Ok(deserialized_msg)
    }

    pub(crate) async fn notify_other(&self, other: impl ToSocketAddrs) -> Result<(), NodeError> {
        //self.send_message(&Message::new_notify(), other).await
        todo!()
    }

    async fn register_node(&self, addr: SocketAddr) -> Result<(), NodeError> {
        self.known_nodes.lock().await.push(addr);

        Ok(())
    }

    pub(crate) async fn broadcast_message(&self, message: &NodeMessage) -> Result<(), NodeError> {
        for addr in self.known_nodes.lock().await.iter() {
            self.send_message(message, addr).await?;
        }

        Ok(())
    }

    pub(crate) async fn choose_consensus_value(
        &self,
        my_value: NodeMessage,
    ) -> Result<NodeMessage, NodeError> {
        Ok(my_value)
    }

    // TODO: remove
    async fn get_log(&self) -> Vec<NodeMessage> {
        self.storage.lock().await.clone()
    }

    pub async fn pending_forever(&self) {
        std::future::pending::<()>().await;
    }
}

#[cfg(feature = "simulation")]
#[cfg(test)]
pub mod tests {
    use crate::proto::message::{node_message, NodeMessage};
    use crate::{proto, Node};
    use network::turmoil;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    pub fn test() {
        let mut matrix = turmoil::Builder::new().build();

        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");

        matrix.host("node_1", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000), "node_1")
                .await
                .unwrap();

            let msg = NodeMessage {
                id: Some(proto::message::Uuid {
                    value: uuid::Uuid::new_v4().into(),
                }),
                kind: node_message::MessageKind::Ordinary as i32,
                data: "hello, world".into(),
            };

            let node_2_addr = ("node_2", 9000);
            let node_3_addr = ("node_3", 9000);

            node.send_message(&msg, node_2_addr).await.unwrap();
            node.send_message(&msg, node_3_addr).await.unwrap();

            Ok(())
        });

        matrix.host("node_2", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000), "node_2")
                .await
                .unwrap();

            node.pending_forever().await;

            Ok(())
        });

        matrix.host("node_3", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000), "node_3")
                .await
                .unwrap();

            node.pending_forever().await;

            Ok(())
        });

        for _ in 0..=100 {
            matrix.step().unwrap();
        }

        matrix.run().unwrap();
    }
}
