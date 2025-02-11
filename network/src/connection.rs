use crate::types::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    SocketAddr, TcpStream,
};
use std::future::Future;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

struct Request<D, Res> {
    data: D,
    response: oneshot::Sender<std::io::Result<Res>>,
    cancel_token: tokio_util::sync::CancellationToken,
}

// TODO: use bounded channels
pub struct Connection {
    tx_write: UnboundedSender<Request<Vec<u8>, ()>>,
    tx_read: UnboundedSender<Request<usize, Vec<u8>>>,
    peer_addr: SocketAddr,
}

impl Connection {
    pub async fn establish(with: SocketAddr) -> Result<Arc<Self>, std::io::Error> {
        let stream = TcpStream::connect(with).await?;

        Self::from_stream(stream)
    }

    pub fn from_stream(stream: TcpStream) -> Result<Arc<Self>, std::io::Error> {
        let peer_addr = stream.peer_addr()?;

        let (reader, writer) = stream.into_split();

        let (tx_write, rx_write) = unbounded_channel();
        tokio::spawn(Self::process_writes(writer, rx_write));

        let (tx_read, rx_read) = unbounded_channel();
        tokio::spawn(Self::process_reads(reader, rx_read));

        Ok(Arc::new(Self {
            tx_write,
            tx_read,
            peer_addr,
        }))
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub async fn read_u64_le(&self) -> std::io::Result<u64> {
        let buf = self.read_buf(8).await?;

        if buf.len() != 8 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        Ok(u64::from_le_bytes(buf.try_into().unwrap()))
    }

    pub fn write_all(&self, data: Vec<u8>) -> impl Future<Output = std::io::Result<()>> {
        let tx = self.tx_write.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            tx.send(Request {
                data,
                response: response_tx,
                cancel_token: child_token,
            })
            .expect("broken connection");

            response_rx.await.expect("broken connection")
        }
    }

    pub fn read_buf(&self, size: usize) -> impl Future<Output = std::io::Result<Vec<u8>>> {
        let tx = self.tx_read.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            tx.send(Request {
                data: size,
                response: response_tx,
                cancel_token: child_token,
            })
            .expect("broken connection");

            response_rx.await.expect("broken connection")
        }
    }

    async fn process_writes(
        mut writer: OwnedWriteHalf,
        mut rx: UnboundedReceiver<Request<Vec<u8>, ()>>,
    ) {
        loop {
            while let Some(Request {
                data,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                tokio::select! {
                    result = writer.write_all(&data) => {
                        let _ = response.send(result);
                    }
                    _ = cancel_token.cancelled() => {}
                }
            }
        }
    }

    async fn process_reads(
        mut reader: OwnedReadHalf,
        mut rx: UnboundedReceiver<Request<usize, Vec<u8>>>,
    ) {
        loop {
            while let Some(Request {
                data: size,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                let mut buf = vec![0; size];

                tokio::select! {
                    result = reader.read_buf(&mut buf) => {
                        match result {
                            Ok(bytes_read) => {
                                buf.truncate(bytes_read);
                                let _ = response.send(Ok(buf));
                            },
                            Err(e) => {
                                 let _ = response.send(Err(e));
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {}
                }
            }
        }
    }
}
