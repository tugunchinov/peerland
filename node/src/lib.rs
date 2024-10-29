mod error;

use crate::error::NodeError;
use network::types::{ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use tokio::sync::Mutex;

// TODO: create struct
type Message = String;

pub struct Node {
    socket: Arc<UdpSocket>,

    // TODO: make service
    storage: Mutex<Vec<Message>>,
}

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Arc<Self>, NodeError> {
        let socket = Arc::new(UdpSocket::bind(my_address).await.expect("TODO: make error"));

        let node = Arc::new(Self {
            socket,
            storage: Mutex::new(vec![]),
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen_messages().await });
        }

        Ok(node)
    }

    async fn listen_messages(self: Arc<Self>) {
        loop {
            match self.recv_message().await {
                Ok(msg) => {
                    self.storage.lock().await.push(msg);
                }
                Err(_e) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub async fn send_message(
        &self,
        msg: Message,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        self.socket
            .send_to(msg.as_bytes(), to)
            .await
            .expect("failed sending message");

        Ok(())
    }

    async fn recv_message(&self) -> Result<Message, NodeError> {
        let mut message_bytes = vec![0; 2048];
        let (_bytes_received, _sender) = self
            .socket
            .recv_from(&mut message_bytes)
            .await
            .expect("TODO: make error");

        let message = String::from_utf8(message_bytes).expect("TODO: make error");

        Ok(message)
    }

    pub async fn list_known_nodes(&self) -> Result<Vec<impl ToSocketAddrs>, NodeError> {
        // TODO: make discovery service
        let node_1_address = "127.0.0.1:59875";
        let node_2_address = "127.0.0.2:59876";
        //let node_3_address = "0.0.0.0:59878";

        Ok(vec![node_1_address, node_2_address])
    }

    pub async fn choose_consensus_value(&self, my_value: Message) -> Result<Message, NodeError> {
        for node in self.list_known_nodes().await? {
            self.send_message(my_value.clone(), node).await?;
        }

        Ok(my_value)
    }

    // TODO: remove
    pub async fn get_log(&self) -> Vec<Message> {
        self.storage.lock().await.clone()
    }
}

#[cfg(test)]
#[cfg(feature = "simulation")]
mod tests {
    use crate::Node;
    use network::turmoil;
    use network::turmoil::IpVersion;
    use tracing::instrument::WithSubscriber;
    use tracing::trace;

    #[test]
    fn test() {
        let mut matrix = turmoil::Builder::default().build();

        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");

        matrix.host("127.1.1.1", || async {
            let node = Node::new(("127.1.1.1", 44444))
                .await
                .expect("TODO: make error");

            node.send_message("hello from node 1".to_string(), ("127.1.1.2", 44444))
                .await
                .unwrap();

            trace!("node 1 sent message");

            let msg = node.recv_message().await.unwrap();

            trace!(message = ?msg, "node 1 received message");

            Ok(())
        });

        matrix.host("127.1.1.2", || async {
            let node = Node::new(("127.1.1.2", 44444))
                .await
                .expect("TODO: make error");

            node.send_message("hello from node 2".to_string(), ("127.1.1.1", 44444))
                .await
                .unwrap();

            trace!("node 2 sent message");

            let msg = node.recv_message().await.unwrap();

            trace!(message = ?msg, "node 2 received message");

            Ok(())
        });

        matrix.run().expect("failed");
    }
}
