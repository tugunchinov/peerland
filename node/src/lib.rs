mod error;
mod proto;
mod sync;
mod time;

use crate::error::NodeError;
use crate::proto::message::node_message;
use crate::proto::*;
use crate::time::*;
use network::types::*;
use prost::Message;
use proto::message::NodeMessage;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub struct Node<T: SystemTimeProvider> {
    id: u32,

    // TODO: use custom protocol over UDP?
    socket: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<NodeMessage>>,

    // TODO: better
    known_nodes: Mutex<Vec<SocketAddr>>,

    system_time_provider: T,
    // TODO: use trait
    logical_time_provider: LamportClock,
}

impl<T: SystemTimeProvider> Node<T> {
    pub async fn new(
        id: u32,
        addr: impl ToSocketAddrs,
        time_provider: T,
    ) -> Result<Arc<Self>, NodeError> {
        let socket = TcpListener::bind(addr).await?;

        let logical_time_provider = LamportClock::new(id);

        let node = Arc::new(Self {
            id,
            socket,
            storage: Mutex::new(vec![]),
            known_nodes: Mutex::new(vec![]),
            system_time_provider: time_provider,
            logical_time_provider,
        });

        {
            let node = Arc::clone(&node);
            tokio::spawn(async move { node.listen_messages().await });
        }

        Ok(node)
    }

    async fn listen_messages(self: Arc<Self>) {
        tracing::info!("start listening messages");
        loop {
            match self.recv_message().await {
                Ok(msg) => {
                    let now: prost_types::Timestamp = self.system_time_provider.now_millis().into();
                    tracing::info!(
                        now = ?now,
                        msg_ts = ?msg.ts,
                        msg_lt = ?msg.lt,
                        "received message"
                    );
                    if let Some(node_message::Lt::LamportClock(msg_lt)) = msg.lt {
                        self.logical_time_provider.adjust_timestamp(msg_lt.lt)
                    } else {
                        tracing::warn!(msg_lt = ?msg.lt, "unknown logical time format");
                    }
                    self.storage.lock().await.push(msg);
                }
                Err(e) => {
                    tracing::error!(error = ?e, "failed receiving message");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub(crate) async fn send_message<B: AsRef<[u8]>>(
        &self,
        msg: B,
        to: impl ToSocketAddrs,
    ) -> Result<(), NodeError> {
        let ts = self.system_time_provider.now_millis().into();
        tracing::info!(timestamp = ?ts, "sending message");

        let msg = NodeMessage {
            id: Some(message::Uuid {
                value: uuid::Uuid::new_v4().into(),
            }),
            kind: message::node_message::MessageKind::Ordinary as i32,
            ts: Some(ts),
            lt: Some(message::node_message::Lt::LamportClock(
                self.logical_time_provider.next_timestamp().into(),
            )),
            data: msg.as_ref().to_vec(),
        };

        let serialized_msg = msg.encode_to_vec();
        let mut stream = TcpStream::connect(to).await?;
        stream.write_all(&serialized_msg).await?;
        Ok(())
    }

    async fn recv_message(&self) -> Result<NodeMessage, NodeError> {
        const MAX_MSG_SIZE_BYTES: usize = 8 * 1024;
        let mut buf = Vec::with_capacity(MAX_MSG_SIZE_BYTES);
        let (mut stream, _addr) = self.socket.accept().await?;
        loop {
            let bytes_read = stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
        }

        let deserialized_msg = NodeMessage::decode(buf.as_slice())?;
        Ok(deserialized_msg)
    }

    pub async fn pending_forever(&self) {
        std::future::pending::<()>().await;
    }
}

#[cfg(feature = "simulation")]
#[cfg(test)]
pub mod tests {
    use crate::time::BrokenUnixTimeProvider;
    use crate::Node;
    use network::turmoil;
    use rand::prelude::SliceRandom;
    use std::future::Future;
    use std::net::{IpAddr, Ipv4Addr};
    use std::pin::Pin;

    #[test]
    pub fn test() {
        use rand::SeedableRng;

        let rng = rand::rngs::StdRng::seed_from_u64(35353);
        let boxed_rng = Box::new(rng.clone());

        let mut matrix = turmoil::Builder::new().build_with_rng(boxed_rng);

        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");

        let node_names = ["node_1", "node_2", "node_3"];

        for (i, node_name) in node_names.iter().enumerate() {
            matrix.host(
                *node_name,
                configure_node(node_names.to_vec(), i, rng.clone(), 100),
            );
        }

        for _ in 0..=100_000 {
            matrix.step().unwrap();
        }

        matrix.run().unwrap();
    }

    fn configure_node<S: AsRef<str>, T: rand::Rng + Clone + Send + 'static>(
        nodes_name: Vec<S>,
        node_idx: usize,
        rng: T,
        spam_msg_cnt: u64,
    ) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
        move || {
            let mut rng = rng.clone();

            let mut other_nodes = Vec::with_capacity(nodes_name.len());
            for (i, node_name) in nodes_name.iter().enumerate() {
                if i != node_idx {
                    other_nodes.push((node_name.as_ref().to_string(), 9000));
                }
            }

            Box::pin(async move {
                let time_provider = BrokenUnixTimeProvider::new(rng.clone());

                let node = Node::new(
                    node_idx as u32,
                    (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                    time_provider,
                )
                .await
                .unwrap();

                // start spaming
                for i in 0..spam_msg_cnt {
                    let msg = format!("hello from {node_idx}: {i}");

                    let recipients_cnt = (rand::random::<usize>() % other_nodes.len()).max(1);

                    tracing::info!(
                        spam_msg_cnt = ?spam_msg_cnt,
                        recipients_cnt = %recipients_cnt,
                        "starting spaming"
                    );

                    for _ in 0..recipients_cnt {
                        let recipient = other_nodes.choose(&mut rng).unwrap();

                        node.send_message(&msg, recipient).await.unwrap();
                    }
                }

                tracing::warn!("finished spaming. pending forever...");

                node.pending_forever().await;

                Ok(())
            })
        }
    }
}
