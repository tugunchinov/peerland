use crate::proto::message::NodeMessage;
use crate::{time, Node};
use network::discovery::Discovery;
use prost::Message;
use rand::Rng;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub(crate) async fn gossip(&self, msg: NodeMessage, level: usize) {
        let serialized_msg = msg.encode_to_vec();
        let mut entropy = self.entropy.clone();
        let nodes = self
            .discovery
            .get_random_nodes(level, &mut entropy)
            .into_iter()
            .collect::<Vec<_>>();
        for node in nodes {
            if let Err(e) = self.send_message_int(&serialized_msg, node).await {
                tracing::error!(error = %e, "failed sending message");
            }
        }
    }
}
