use crate::communication::proto::message::*;
use crate::error::NodeError;
use crate::{time, Node};
use rand::Rng;
use std::sync::Arc;

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    // TODO: better

    pub(crate) async fn broadcast_reliably<B: AsRef<[u8]>>(
        self: &Arc<Self>,
        msg: B,
    ) -> Result<(), NodeError> {
        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Reliable.into());

        let node_msg = self.create_node_message(msg, msg_kind);

        self.process_message(self.socket.local_addr()?, node_msg)
            .await
    }

    pub(crate) async fn gossip<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B) -> Result<(), NodeError> {
        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Gossip.into());

        let node_msg = self.create_node_message(msg, msg_kind);

        self.process_message(self.socket.local_addr()?, node_msg)
            .await
    }
}
