mod error;
mod message;

use crate::error::NodeError;
use crate::message::Message;
use network::types::*;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Node {
    socket: UdpSocket,

    // TODO: make service
    storage: Mutex<Vec<Message>>,

    // TODO: better
    known_nodes: Mutex<Vec<SocketAddr>>,
}

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Arc<Self>, NodeError> {
        let socket = UdpSocket::bind(my_address).await?;

        let node = Arc::new(Self {
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
        msg: &Message,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        self.socket.send_to(msg.pack()?.as_slice(), to).await?;

        Ok(())
    }

    async fn recv_message(&self) -> Result<Message, NodeError> {
        let mut buf = [0; 2 * Message::max_size()];
        let (bytes_received, _sender) = self.socket.recv_from(&mut buf).await?;

        tracing::trace!(buffer = ?buf, bytes_received = ?bytes_received);

        let msg = Message::unpack(&buf[..bytes_received])?;

        Ok(msg)
    }

    pub(crate) async fn notify_other(&self, other: impl ToSocketAddrs) -> Result<(), NodeError> {
        self.send_message(&Message::new_notify(), other).await
    }

    async fn register_node(&self, addr: SocketAddr) -> Result<(), NodeError> {
        self.known_nodes.lock().await.push(addr);

        Ok(())
    }

    pub(crate) async fn broadcast_message(&self, message: &Message) -> Result<(), NodeError> {
        for addr in self.known_nodes.lock().await.iter() {
            self.send_message(message, addr).await?;
        }

        Ok(())
    }

    pub(crate) async fn choose_consensus_value(
        &self,
        my_value: Message,
    ) -> Result<Message, NodeError> {
        Ok(my_value)
    }

    // TODO: remove
    async fn get_log(&self) -> Vec<Message> {
        self.storage.lock().await.clone()
    }

    pub async fn pending_forever(&self) {
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
#[cfg(feature = "simulation")]
pub mod tests {
    use crate::Node;
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
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000))
                .await
                .unwrap();

            Ok(())
        });

        matrix.host("node_2", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000))
                .await
                .unwrap();

            node.pending_forever().await;

            Ok(())
        });

        matrix.host("node_3", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000))
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
