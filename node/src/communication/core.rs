use crate::communication::proto;
use crate::communication::proto::message::NodeMessage;
use crate::error::NodeError;
use crate::{time, Node, DEFAULT_TIMEOUT};
use network::types::*;
use prost::Message;
use rand::Rng;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;

impl<
        ST: time::SystemTimeProvider + Send + Sync + 'static,
        LT: time::LogicalTimeProvider + Send + Sync + 'static,
        R: Rng + Send + Sync + 'static,
    > Node<ST, LT, R>
{
    pub(crate) async fn listen(self: &Arc<Self>) {
        tracing::info!(my_id = self.id, "start listening to messages...");

        // let mut tasks = JoinSet::new();

        // TODO: use TCP/Custom Protocol
        loop {
            // match self.socket.accept().await {
            //     Ok((stream, sender)) => {
            //         let this = Arc::clone(self);
            //         let buf = buf[0..bytes].to_vec();
            //
            //         tasks.spawn(async move {
            //             let Ok(deserialized_msg) = NodeMessage::decode(buf.as_slice()) else {
            //                 tracing::error!(%sender, "bad message");
            //                 return;
            //             };
            //
            //             // TODO: add deliver function
            //             this.logical_time_provider
            //                 .adjust_from_message(&deserialized_msg);
            //             this.logical_time_provider.tick();
            //
            //             if let Err(e) = this.process_message(sender, &deserialized_msg).await {
            //                 tracing::error!(error = ?e, "unable to process message");
            //             }
            //         });
            //     }
            //     Err(e) => {
            //         tracing::error!(error = ?e, "failed to accept connection");
            //     }
            // }
            //
            // // TODO: from conf?
            // if tasks.len() > 1000 {
            //     while let Some(Ok(_)) = tasks.join_next().await {}
            // }
        }
    }

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
                while let Err(e) = this.send_serialized_message(&serialized_msg, &node).await {
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
    pub(in crate::communication) async fn establish_connection(
        &self,
        with: &SocketAddr,
    ) -> Result<Arc<TcpStream>, NodeError> {
        tracing::info!(peer = %with, "trying to establish a connection");

        let connection =
            Arc::new(tokio::time::timeout(DEFAULT_TIMEOUT, TcpStream::connect(with)).await??);

        tracing::info!(peer = %with, "connection established");

        self.established_connections
            .lock()
            .insert(*with, Arc::clone(&connection));

        Ok(connection)
    }

    pub(in crate::communication) async fn send_serialized_message(
        &self,
        serialized_msg: &[u8],
        to: &SocketAddr,
    ) -> Result<(), NodeError> {
        tracing::info!(recipient = ?to, "sending message");

        let maybe_stream = self.established_connections.lock().get(to).cloned();

        let stream = match maybe_stream {
            Some(stream) => stream,
            None => {
                tracing::warn!(peer = %to, "connection not found");
                self.establish_connection(to).await?
            }
        };

        let size_bytes = (serialized_msg.len() as u64).to_le_bytes();
        size_bytes.iter().chain(serialized_msg);

        //tokio::time::timeout(DEFAULT_TIMEOUT, stream.w()).await??;

        Ok(())
    }

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
