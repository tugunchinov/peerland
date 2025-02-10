use crate::communication::proto::message::{broadcast, MessageKind, NodeMessage};
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

        {
            let mut processed_messages_guard = self.processed_messages.lock().await;

            if processed_messages_guard.contains(&msg_id) {
                // Processed
                tracing::warn!(message_id = %msg_id, "already processed. skipping.");
                return Ok(());
            };

            processed_messages_guard.insert(msg_id);
        }

        let payload = &msg.payload;
        self.storage.lock().push(payload.clone());

        if let Some(msg_kind) = msg.message_kind {
            match msg_kind {
                MessageKind::Broadcast(b) => {
                    if let Ok(broadcast_type) = b.try_into() {
                        match broadcast_type {
                            // TODO: level from message
                            broadcast::BroadcastType::Gossip => {
                                // let random_nodes = self
                                //     .discovery
                                //     .get_random_nodes(5)
                                //     .into_iter()
                                //     .collect::<Vec<_>>();
                                //
                                // // TODO: better
                                // self.broadcast_to(msg.encode_to_vec(), random_nodes, Some(true))
                                //     .await;
                                todo!()
                            }
                            broadcast::BroadcastType::Reliable => {
                                // let nodes = self
                                //     .discovery
                                //     .list_known_nodes()
                                //     .into_iter()
                                //     .collect::<Vec<_>>();
                                //
                                // // TODO: better
                                // self.broadcast_to(msg.encode_to_vec(), nodes, Some(false))
                                //     .await;
                                todo!()
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
