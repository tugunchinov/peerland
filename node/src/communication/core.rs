use crate::communication::proto;
use crate::communication::proto::message::NodeMessage;
use crate::error::NodeError;
use crate::{time, Node};
use network::discovery::Discovery;
use network::types::*;
use prost::Message;
use rand::Rng;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinSet;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub(in crate::communication) async fn send_serialized_message(
        &self,
        serialized_msg: &[u8],
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        // TODO: from config
        const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(30);

        let mut stream = tokio::time::timeout(TIMEOUT, TcpStream::connect(to)).await??;

        tracing::info!(recipient = ?stream.peer_addr(), "sending message");

        // TODO: From config?
        tokio::time::timeout(
            tokio::time::Duration::from_secs(120),
            stream.write_all(serialized_msg),
        )
        .await??;

        Ok(())
    }

    pub(crate) async fn listen(self: Arc<Self>) {
        // TODO: smarter bound
        const MAX_MSG_SIZE_BYTES: usize = 8 * 1024;

        tracing::info!(my_id = self.id, "start listening to messages...");

        let mut tasks = JoinSet::new();

        loop {
            match self.socket.accept().await {
                Ok((mut stream, sender)) => {
                    let this = Arc::clone(&self);
                    tasks.spawn(async move {
                        let mut buf = Vec::with_capacity(MAX_MSG_SIZE_BYTES);

                        loop {
                            // TODO:
                            let Ok(bytes_read) = stream.read_buf(&mut buf).await else {
                                break;
                            };
                            if bytes_read == 0 {
                                break;
                            }
                        }

                        drop(stream);

                        let Ok(deserialized_msg) = NodeMessage::decode(buf.as_slice()) else {
                            tracing::error!(%sender, "bad message");
                            return;
                        };

                        this.logical_time_provider
                            .adjust_from_message(&deserialized_msg);
                        this.logical_time_provider.tick();

                        if let Err(e) = this.process_message(sender, &deserialized_msg).await {
                            tracing::error!(error = ?e, "unable to process message");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = ?e, "failed to accept connection");
                }
            }

            // TODO: from conf?
            if tasks.len() > 1000 {
                while let Some(Ok(_)) = tasks.join_next().await {}
            }
        }
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
