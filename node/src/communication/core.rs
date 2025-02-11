use crate::communication::proto;
use crate::communication::proto::message::NodeMessage;
use crate::error::NodeError;
use crate::{time, Node, DEFAULT_TIMEOUT};
use network::types::*;
use network::Connection;
use rand::Rng;
use std::sync::Arc;
use tokio::task::JoinSet;

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    pub(crate) async fn accept_connections(self: Arc<Self>) {
        tracing::info!(my_id = self.id, "start accepting connections...");

        loop {
            match self.socket.accept().await {
                Ok((stream, peer)) => match Connection::from_stream(stream) {
                    Ok(connection) => {
                        self.established_connections
                            .lock()
                            .insert(peer, Arc::clone(&connection));

                        // TODO: check if connection already exist
                        // drop old
                        let this = Arc::clone(&self);
                        tokio::spawn(async move {
                            this.process_connection(connection).await;
                        });
                    }
                    Err(e) => {
                        tracing::error!(%peer, error = %e, "connection error")
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "failed accepting connection");
                }
            }
        }
    }

    pub(crate) async fn establish_connection(
        self: &Arc<Self>,
        with: SocketAddr,
    ) -> Result<Arc<Connection>, NodeError> {
        tracing::info!(peer = %with, "trying to establish a connection");

        let connection =
            tokio::time::timeout(DEFAULT_TIMEOUT, Connection::establish(with)).await??;

        tracing::info!(peer = %with, "connection established");

        self.established_connections
            .lock()
            .insert(with, Arc::clone(&connection));

        Ok(connection)
    }

    pub(in crate::communication) async fn send_serialized_message(
        self: &Arc<Self>,
        serialized_msg: &[u8],
        to: SocketAddr,
    ) -> Result<(), NodeError> {
        tracing::info!(recipient = ?to, "sending message");

        let maybe_connection = self.established_connections.lock().get(&to).cloned();

        let connection = match maybe_connection {
            Some(connection) => connection,
            None => return Err(NodeError::UnknownPeer(to)),
        };

        let size_bytes = (serialized_msg.len() as u64).to_le_bytes();
        let msg_with_size = size_bytes
            .iter()
            .cloned()
            .chain(serialized_msg.iter().cloned())
            .collect::<Vec<_>>();

        // TODO: drop connection if timed-out?
        tokio::time::timeout(DEFAULT_TIMEOUT, connection.write_all(msg_with_size)).await??;

        Ok(())
    }

    // TODO: time out anyway
    pub(crate) async fn broadcast_to(
        self: &Arc<Self>,
        serialized_msg: Vec<u8>,
        to: impl Iterator<Item = SocketAddr>,
        skip_failed: Option<bool>,
    ) {
        let skip_failed = skip_failed.unwrap_or(false);

        let mut tasks = JoinSet::new();
        for node in to {
            let serialized_msg = serialized_msg.clone();
            let this = Arc::clone(self);

            tasks.spawn(async move {
                while let Err(e) = this.send_serialized_message(&serialized_msg, node).await {
                    tracing::error!(
                        error = %e,
                        "failed sending message"
                    );

                    if skip_failed {
                        break;
                    }
                }
            });
        }

        tasks.join_all().await;
    }
}

impl<ST: time::SystemTimeProvider, LT: time::LogicalTimeProvider, R: Rng> Node<ST, LT, R> {
    pub(in crate::communication) fn create_node_message<B: AsRef<[u8]>>(
        &self,
        data: B,
        msg_kind: proto::message::MessageKind,
    ) -> NodeMessage {
        let ts = self.system_time_provider.now_millis().into();

        NodeMessage {
            id: Some(uuid::Uuid::new_v4().into()),
            message_kind: Some(msg_kind),
            ts: Some(ts),
            lt: Some(self.logical_time_provider.tick().into()),
            payload: data.as_ref().to_vec(),
        }
    }
}
