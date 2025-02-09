use crate::{time, Node};
use network::discovery::Discovery;
use rand::Rng;
use std::sync::Arc;

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, D: Discovery, R: Rng>
    Node<ST, LT, D, R>
{
    pub(crate) async fn gossip<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B) {
        use crate::communication::proto::message::*;

        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Gossip.into());
        let node_msg = self.create_node_message(msg, msg_kind);

        // TODO: better flow
        if let Err(e) = self
            .process_message(self.socket.local_addr().unwrap(), &node_msg)
            .await
        {
            tracing::error!(error = %e, "failed to process message. refuse broadcasting it");
        }
    }
}
