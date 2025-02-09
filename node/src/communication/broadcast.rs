use crate::communication::proto::message::MessageKind;
use crate::{time, Node};
use network::discovery::Discovery;
use rand::Rng;
use std::sync::Arc;

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, D: Discovery, R: Rng>
    Node<ST, LT, D, R>
{
    // TODO: better

    pub(crate) async fn broadcast_reliably<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B) {
        use crate::communication::proto::message::*;
        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Reliable.into());
        self.deliver(msg, msg_kind).await;
    }

    pub(crate) async fn gossip<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B) {
        use crate::communication::proto::message::*;
        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Gossip.into());
        self.deliver(msg, msg_kind).await;
    }

    async fn deliver<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B, msg_kind: MessageKind) {
        let node_msg = self.create_node_message(msg, msg_kind);

        if let Err(e) = self
            .process_message(self.socket.local_addr().unwrap(), &node_msg)
            .await
        {
            tracing::error!(error = %e, "failed processing message");
        }
    }
}
