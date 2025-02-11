use crate::communication::proto::message::{broadcast, MessageKind, NodeMessage};
use crate::error::NodeError;
use crate::{time, Node};
use network::types::SocketAddr;
use prost::Message;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::ops::DerefMut;
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
        msg: NodeMessage,
    ) -> Result<(), NodeError> {
        // TODO:
        let Some(Ok(msg_id)) = msg.id.as_ref().map(|id| id.try_into()) else {
            tracing::error!(sender = %from, "bad message id. skipping.");
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
                    let Ok(broadcast_type) = b.try_into() else {
                        unreachable!()
                    };

                    let mut all_peers = self
                        .established_connections
                        .lock()
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>();

                    match broadcast_type {
                        // TODO: broadcast number from message
                        broadcast::BroadcastType::Gossip => {
                            all_peers.shuffle(self.entropy.lock().deref_mut());
                            all_peers.truncate(3);

                            // TODO: better
                            self.broadcast_to(
                                msg.encode_to_vec(),
                                all_peers.into_iter(),
                                Some(true),
                            )
                            .await;
                        }
                        broadcast::BroadcastType::Reliable => {
                            // TODO: better
                            self.broadcast_to(
                                msg.encode_to_vec(),
                                all_peers.into_iter(),
                                Some(false),
                            )
                            .await;
                        }
                    }
                }
                MessageKind::Addressed(_a) => {
                    // TODO:
                    tracing::warn!("not implemented");
                }
            }
        } else {
            tracing::error!(msg_id = ?msg.id, "unknown message kind");
        }

        Ok(())
    }
}
