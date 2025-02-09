use crate::communication::proto::message::{broadcast, MessageKind, NodeMessage};
use crate::error::NodeError;
use crate::{time, Node};
use network::discovery::Discovery;
use network::types::SocketAddr;
use prost::Message;
use rand::Rng;
use std::sync::Arc;

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, D: Discovery, R: Rng>
    Node<ST, LT, D, R>
{
    pub(crate) async fn process_message(
        self: &Arc<Self>,
        from: SocketAddr,
        msg: &NodeMessage,
    ) -> Result<(), NodeError> {
        // TODO:
        let Some(Ok(msg_id)) = msg.id.as_ref().map(|id| id.try_into()) else {
            tracing::warn!(sender = %from, "bad message id. skipping.");
            return Ok(());
        };

        if self.processed_messages.lock().await.contains(&msg_id) {
            // Processed
            tracing::warn!(message_id = %msg_id, "already processed. skipping.");
            return Ok(());
        };

        self.processed_messages.lock().await.insert(msg_id);

        let payload = &msg.payload;

        self.storage.lock().await.push(payload.clone());

        if let Some(msg_kind) = msg.message_kind {
            match msg_kind {
                MessageKind::Broadcast(b) => {
                    if let Ok(broadcast_type) = b.try_into() {
                        match broadcast_type {
                            // TODO: level from message
                            broadcast::BroadcastType::Gossip => {
                                let random_nodes = self.discovery.get_random_nodes(3);
                                // TODO: better
                                Self::broadcast_to(
                                    msg.encode_to_vec(),
                                    random_nodes.into_iter().collect::<Vec<_>>(),
                                )
                                .await;
                            }
                        }
                    } else {
                        unreachable!()
                    }
                }
                MessageKind::Addressed(_a) => {
                    // TODO:
                    tracing::warn!("not implemented");
                }
            }
        } else {
            tracing::warn!(msg_id = ?msg.id, "unknown message kind");
        }

        Ok(())
    }
}
