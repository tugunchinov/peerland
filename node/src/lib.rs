mod error;

use crate::error::NodeError;
use network::types::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

// TODO: create struct
type Message = String;

pub struct Node {
    socket: UdpSocket,

    // TODO: make service
    storage: Mutex<Vec<Message>>,
}

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Arc<Self>, NodeError> {
        let socket = UdpSocket::bind(my_address).await.expect("TODO: make error");

        let node = Arc::new(Self {
            socket,
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
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        self.socket.send_to(msg.as_bytes(), to).await.unwrap();

        Ok(())
    }

    async fn recv_message(&self) -> Result<Message, NodeError> {
        let mut buf = vec![0; 128];
        let (bytes, addr) = self.socket.recv_from(&mut buf).await.unwrap();

        tracing::warn!(bytes = ?bytes, from = ?addr, "received");

        let msg = String::from_utf8(buf).unwrap();

        Ok(msg)
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

//#[cfg(test)]
#[cfg(feature = "simulation")]
pub mod tests {
    use crate::Node;
    use network::turmoil;
    use network::turmoil::net::UdpSocket;
    use network::types::{TcpListener, TcpStream};
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    //#[test]
    pub fn test() {
        let mut matrix = turmoil::Builder::new()
            .max_message_latency(tokio::time::Duration::from_millis(100))
            .build();

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

            tracing::warn!("start listening");

            let msg = node.recv_message().await.unwrap();

            tracing::warn!(msg = ?msg, "received");

            Ok(())
        });

        matrix.host("node_2", || async {
            let node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000))
                .await
                .unwrap();

            tracing::warn!("binded");

            node.send_message("hello".to_string(), ("node_1", 9000))
                .await
                .unwrap();

            tracing::warn!("sent");

            Ok(())
        });

        for _ in 0..=100 {
            matrix.step().unwrap();
        }

        //println!("host: {}", matrix.is_host_running("127.0.0.1"));

        matrix.run().unwrap();
    }
}
