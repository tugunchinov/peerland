mod error;

use crate::error::NodeError;
use tokio::net::{ToSocketAddrs, UdpSocket};

pub struct Node {
    socket: UdpSocket,
}

// TODO: create struct
type Message = String;

impl Node {
    pub async fn new(my_address: impl ToSocketAddrs) -> Result<Self, NodeError> {
        let socket = UdpSocket::bind(my_address).await.expect("TODO: make error");

        Ok(Self { socket })
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
}
