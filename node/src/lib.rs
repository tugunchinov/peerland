mod error;

use crate::error::NodeError;
use network::types::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

// TODO: create struct
type Message = String;

pub struct Node {
    listener: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<Message>>,
}

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Arc<Self>, NodeError> {
        let listener = TcpListener::bind(my_address)
            .await
            .expect("TODO: make error");

        let node = Arc::new(Self {
            listener,
            storage: Mutex::new(vec![]),
        });
        //
        // {
        //     let node = Arc::clone(&node);
        //     tokio::spawn(async move { node.listen_messages().await });
        // }

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
        to: impl ToSocketAddrs + std::fmt::Debug + Clone,
    ) -> Result<(), NodeError> {
        tracing::trace!(to = ?to, "trying to connect");

        let mut tcp_stream = TcpStream::connect(to.clone())
            .await
            .expect("TODO: make error");

        tracing::trace!(to = ?to, "connected");

        tcp_stream
            .write_all(msg.as_bytes())
            .await
            .expect("failed sending message");

        tcp_stream
            .flush()
            .await
            .expect("failed flushing TCP stream");

        Ok(())
    }

    async fn recv_message(&self) -> Result<Message, NodeError> {
        let mut message_bytes = vec![0; 32];

        tracing::trace!("trying to receive bytes");

        let (mut stream, sender) = self.listener.accept().await.expect("TODO: make error");

        tracing::trace!(sender = ?sender);
        stream
            .read_buf(&mut message_bytes)
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
        // for node in self.list_known_nodes().await? {
        //     self.send_message(my_value.clone(), node).await?;
        // }

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

    #[test]
    fn test() {
        let mut matrix = turmoil::Builder::default()
            .tick_duration(tokio::time::Duration::from_secs(60))
            .simulation_duration(tokio::time::Duration::from_secs(60 * 60 * 60))
            .build();

        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");

        matrix.host("127.0.0.1", || async {
            let node = Node::new(("127.0.0.1", 44444))
                .await
                .expect("TODO: make error");

            tracing::warn!("node 1 init");

            node.send_message("hello from node 1".to_string(), ("0.0.0.0", 44444))
                .await
                .unwrap();

            tracing::warn!("node 1 sent message");

            let msg = node.recv_message().await.unwrap();

            tracing::warn!(message = ?msg, "node 1 received message");

            Ok(())
        });

        matrix.host("0.0.0.0", || async {
            let node = Node::new(("0.0.0.0", 44444))
                .await
                .expect("TODO: make error");

            tracing::warn!("node 2 init");

            node.send_message("hello from node 2".to_string(), ("127.0.0.1", 44444))
                .await
                .unwrap();

            tracing::warn!("node 2 sent message");

            let msg = node.recv_message().await.unwrap();

            tracing::warn!(message = ?msg, "node 2 received message");

            Ok(())
        });

        for _ in 0..=50 {
            matrix.step().unwrap();
        }

        //println!("host: {}", matrix.is_host_running("127.0.0.1"));

        //matrix.run().expect("failed");
    }
}
