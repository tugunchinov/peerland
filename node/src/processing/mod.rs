use crate::communication::proto::message::{broadcast, MessageKind, NodeMessage};
use crate::error::NodeError;
use crate::{time, Node};
use network::discovery::Discovery;
use network::types::SocketAddr;
use rand::Rng;
use std::sync::Arc;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub(crate) async fn process_message(
        self: Arc<Self>,
        from: SocketAddr,
        msg: NodeMessage,
    ) -> Result<(), NodeError> {
        // TODO:
        if from == self.socket.local_addr()? {
            tracing::warn!("skip message from myself");
            return Ok(());
        }

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

        let payload = msg.payload;

        self.storage.lock().await.push(payload.clone());

        if let Some(msg_kind) = msg.message_kind {
            match msg_kind {
                MessageKind::Broadcast(b) => {
                    if let Ok(broadcast_type) = b.try_into() {
                        match broadcast_type {
                            // TODO: level from message
                            broadcast::BroadcastType::Gossip => {
                                self.gossip(payload, 2).await;
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
