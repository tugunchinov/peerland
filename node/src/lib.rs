mod error;
mod net_types;

use crate::error::NodeError;
use crate::net_types::{ToSocketAddrs, UdpSocket};

// TODO: create struct
type Message = String;

pub struct Node {
    socket: UdpSocket,

    // TODO: make service
    storage: Vec<Message>,
}

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Self, NodeError> {
        let socket = UdpSocket::bind(my_address).await.expect("TODO: make error");

        Ok(Self {
            socket,
            storage: vec![],
        })
    }

    pub async fn send_message(
        &self,
        msg: Message,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        self.socket
            .send_to(msg.as_bytes(), to)
            .await
            .expect("TODO: make error");

        Ok(())
    }

    pub async fn recv_message(&self) -> Result<Message, NodeError> {
        let mut message_bytes = vec![0; 2048];
        let (_, sender) = self
            .socket
            .recv_from(&mut message_bytes)
            .await
            .expect("TODO: make error");

        // TODO: log
        println!("SENDER: {sender}");

        let message = String::from_utf8(message_bytes).expect("TODO: make error");

        Ok(message)
    }

    pub async fn list_known_nodes(&self) -> Result<Vec<impl ToSocketAddrs>, NodeError> {
        // TODO: make discovery service
        let node_1_address = "0.0.0.0:9875";
        let node_2_address = "0.0.0.0:9876";
        let node_3_address = "0.0.0.0:9878";

        Ok(vec![node_1_address, node_2_address, node_3_address])
    }

    pub async fn choose_consensus_value(
        &mut self,
        my_value: Message,
    ) -> Result<&Message, NodeError> {
        self.storage.push(my_value);

        Ok(&self.storage.last().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::Node;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::Duration;

    #[test]
    fn test() {
        let mut matrix = turmoil::Builder::new()
            .simulation_duration(Duration::from_secs(100))
            .build();

        matrix.host("node1", || async {
            let mut node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 4411))
                .await
                .expect("TODO: make error");

            for i in 0..10 {
                let my_msg = format!("node1_msg{i}");

                println!("node 1 msg: {my_msg}");

                node.choose_consensus_value(my_msg).await.expect("failed");
            }

            println!("node 1 done");

            tokio::time::sleep(Duration::from_secs(50)).await;

            Ok(())
        });

        matrix.host("node2", || async {
            let mut node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 3399))
                .await
                .expect("TODO: make error");

            for i in 0..10 {
                let my_msg = format!("node2_msg{i}");
                node.choose_consensus_value(my_msg).await.expect("failed");
            }

            Ok(())
        });

        matrix.host("node3", || async {
            let mut node = Node::new((IpAddr::from(Ipv4Addr::UNSPECIFIED), 2288))
                .await
                .expect("TODO: make error");

            for i in 0..10 {
                let my_msg = format!("node3_msg{i}");
                node.choose_consensus_value(my_msg).await.expect("failed");
            }

            Ok(())
        });

        matrix.run().expect("failed")
    }
}
