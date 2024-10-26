mod error;

use crate::error::NodeError;
use tokio::net::{ToSocketAddrs, UdpSocket};

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
