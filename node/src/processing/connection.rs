use crate::communication::proto::message::NodeMessage;
use crate::{time, Node};
use network::Connection;
use prost::Message;
use rand::Rng;
use std::sync::Arc;

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    pub(crate) async fn process_connection(self: &Arc<Self>, connection: Arc<Connection>) {
        let peer = connection.peer_addr();
        loop {
            match connection.read_u64_le().await {
                Ok(msg_size) => match connection.read_exact(msg_size as usize).await {
                    Ok(serialized_msg) => {
                        let Ok(deserialized_msg) = NodeMessage::decode(serialized_msg.as_slice())
                        else {
                            tracing::error!(%peer, "bad message");
                            continue;
                        };

                        self.logical_time_provider
                            .adjust_from_message(&deserialized_msg);
                        self.logical_time_provider.tick();

                        if let Err(e) = self.process_message(peer, deserialized_msg).await {
                            tracing::error!(
                                %peer,
                                error = %e,
                                "failed processing message"
                            );
                        }
                    }
                    Err(e) => {
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::ConnectionRefused
                                | std::io::ErrorKind::ConnectionReset
                        ) {
                            tracing::warn!(
                                %peer,
                                error = %e,
                                "connection closed"
                            );

                            self.established_connections.lock().remove(&peer);

                            return;
                        }

                        tracing::error!(
                            %peer,
                            error = %e,
                            "failed reading message"
                        );
                    }
                },
                Err(e) => {
                    tracing::error!(
                        %peer,
                        error = %e,
                        "failed reading message size"
                    );
                }
            }
        }
    }
}
