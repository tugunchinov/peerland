use crate::{time, Node};
use network::discovery::Discovery;
use rand::Rng;
use std::sync::Arc;
use tokio::task::JoinSet;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub(crate) async fn gossip<B: AsRef<[u8]>>(self: &Arc<Self>, msg: B, level: usize) {
        use crate::communication::proto::message::*;

        let msg_kind = MessageKind::Broadcast(broadcast::BroadcastType::Gossip.into());
        let serialized_msg = self.create_serialized_node_message(msg, msg_kind);

        let mut entropy = self.entropy.clone();
        let nodes = self
            .discovery
            .get_random_nodes(level, &mut entropy)
            .into_iter()
            .collect::<Vec<_>>();

        let mut tasks = JoinSet::new();
        for node in nodes {
            let this = Arc::clone(&self);
            let serialized_msg = serialized_msg.clone();

            tasks.spawn(async move {
                if let Err(e) = this.send_serialized_message(&serialized_msg, node).await {
                    tracing::error!(
                        error = %e,
                        "failed sending message"
                    );
                }
            });
        }

        tasks.join_all().await;
    }
}
