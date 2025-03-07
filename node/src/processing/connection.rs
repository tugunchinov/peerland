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
        let mut buf = vec![0u8; 1024];
        loop {
            match connection.read_u64_le().await {
                Ok(msg_size) => {
                    buf.resize(msg_size as usize, 0);

                    match connection.read_exact(std::mem::take(&mut buf)).await {
                        Ok(serialized_msg) => {
                            let Ok(mut deserialized_msg) =
                                NodeMessage::decode(serialized_msg.as_slice())
                            else {
                                tracing::error!(%peer, "bad message");
                                continue;
                            };

                            self.logical_time_provider
                                .adjust_from_message(&deserialized_msg);
                            deserialized_msg.lt = Some(self.logical_time_provider.tick().into());

                            if let Err(e) = self.process_message(peer, deserialized_msg).await {
                                tracing::error!(
                                    %peer,
                                    error = %e,
                                    "failed processing message"
                                );
                            }

                            buf = serialized_msg;
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
                    }
                }
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
