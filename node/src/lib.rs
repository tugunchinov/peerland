mod error;
mod proto;
mod sync;
mod time;

use crate::error::NodeError;
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

    socket: TcpListener,

    // TODO: make service
    storage: Mutex<Vec<NodeMessage>>,

    // TODO: better
    known_nodes: Mutex<Vec<SocketAddr>>,

    system_time_provider: T,
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
                        "received message"
                    );
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
            tracing::info!(bytes_read = ?bytes_read, buffer_len = ?buf.len());
        }

        let deserialized_msg = NodeMessage::decode(buf.as_slice())?;
        Ok(deserialized_msg)
    }

    async fn register_node(&self, addr: SocketAddr) -> Result<(), NodeError> {
        self.known_nodes.lock().await.push(addr);

        Ok(())
    }

    pub(crate) async fn choose_consensus_value(
        &self,
        my_value: NodeMessage,
    ) -> Result<NodeMessage, NodeError> {
        Ok(my_value)
    }

    // TODO: use trait instead
    fn get_next_lt(&self) -> LamportClockUnit {
        self.logical_time_provider.next_timestamp()
    }

    // TODO: remove
    async fn get_log(&self) -> Vec<NodeMessage> {
        self.storage.lock().await.clone()
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
    use std::net::{IpAddr, Ipv4Addr};

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

        {
            let rng = rng.clone();

            matrix.host("node_1", move || {
                let rng = rng.clone();

                async move {
                    let time_provider = BrokenUnixTimeProvider::new(rng);

                    let node = Node::new(
                        1,
                        (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                        time_provider,
                    )
                    .await
                    .unwrap();

                    let node_2_addr = ("node_2", 9000);
                    let node_3_addr = ("node_3", 9000);

                    node.send_message("hello, world!", node_2_addr)
                        .await
                        .unwrap();
                    node.send_message("hello, world!", node_3_addr)
                        .await
                        .unwrap();

                    Ok(())
                }
            });
        }

        {
            let rng = rng.clone();

            matrix.host("node_2", move || {
                let rng = rng.clone();
                async move {
                    let time_provider = BrokenUnixTimeProvider::new(rng);

                    let node = Node::new(
                        2,
                        (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                        time_provider,
                    )
                    .await
                    .unwrap();

                    node.pending_forever().await;

                    Ok(())
                }
            });
        }

        {
            let rng = rng.clone();

            matrix.host("node_3", move || {
                let rng = rng.clone();
                async move {
                    let time_provider = BrokenUnixTimeProvider::new(rng);

                    let node = Node::new(
                        3,
                        (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                        time_provider,
                    )
                    .await
                    .unwrap();

                    node.pending_forever().await;

                    Ok(())
                }
            });
        }

        for _ in 0..=100 {
            matrix.step().unwrap();
        }

        matrix.run().unwrap();
    }
}
