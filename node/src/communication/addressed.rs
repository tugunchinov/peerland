use crate::error::NodeError;
use crate::{time, Node};
use network::types::SocketAddr;
use prost::Message;
use rand::Rng;
use std::sync::Arc;

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    pub async fn send_to<B: AsRef<[u8]>>(
        self: &Arc<Self>,
        to: SocketAddr,
        msg: B,
    ) -> Result<(), NodeError> {
        use crate::communication::proto::message::*;

        let node_msg = self.create_node_message(
            msg,
            MessageKind::Addressed(addressed::MessageType::Ordinary.into()),
        );

        self.send_serialized_message(&node_msg.encode_to_vec(), to)
            .await
    }
}
