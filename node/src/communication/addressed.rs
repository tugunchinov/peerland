use crate::error::NodeError;
use crate::{time, Node};
use network::discovery::Discovery;
use network::types::ToSocketAddrs;
use prost::Message;
use rand::Rng;

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, D: Discovery, R: Rng>
    Node<ST, LT, D, R>
{
    pub async fn send_to<B: AsRef<[u8]>>(
        &self,
        to: impl ToSocketAddrs,
        msg: B,
    ) -> Result<(), NodeError> {
        use crate::communication::proto::message::*;

        let node_msg = self.create_node_message(
            msg,
            MessageKind::Addressed(addressed::MessageType::Ordinary.into()),
        );

        Self::send_serialized_message(&node_msg.encode_to_vec(), to).await
    }
}
