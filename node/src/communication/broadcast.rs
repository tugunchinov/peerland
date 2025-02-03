use crate::{time, Node};
use network::discovery::Discovery;
use rand::Rng;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub(crate) async fn gossip<B: AsRef<[u8]>>(&self, msg: B, level: usize) {
        use crate::proto::message::*;

        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Gossip.into());
        let serialized_msg = self.create_serialized_node_message(msg, msg_kind);

        let mut entropy = self.entropy.clone();
        let nodes = self
            .discovery
            .get_random_nodes(level, &mut entropy)
            .into_iter()
            .collect::<Vec<_>>();
        for node in nodes {
            if let Err(e) = self.send_serialized_message(&serialized_msg, node).await {
                tracing::error!(
                    error = %e,
                    "failed sending message"
                );
            }
        }
    }
}
