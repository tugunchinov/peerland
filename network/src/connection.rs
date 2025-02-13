use crate::types::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    SocketAddr, TcpStream,
};
use std::future::Future;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

struct Request<C, Res> {
    command: C,
    response: oneshot::Sender<std::io::Result<Res>>,
    cancel_token: tokio_util::sync::CancellationToken,
}

enum ReadCommand {
    ReadBuf(Vec<u8>),
    ReadExact(Vec<u8>),
    ReadU64LE,
}

enum WriteCommand {
    WriteAll(Vec<u8>),
}

// TODO: use bounded channels
pub struct Connection {
    tx_write: Sender<Request<WriteCommand, ()>>,
    tx_read: Sender<Request<ReadCommand, (u64, Option<Vec<u8>>)>>,
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

        // TODO: bounds from config/args?

        let (tx_write, rx_write) = channel(100_000);
        tokio::spawn(Self::process_writes(writer, rx_write));

        let (tx_read, rx_read) = channel(100_000);
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

    pub fn read_u64_le(&self) -> impl Future<Output = std::io::Result<u64>> {
        let tx = self.tx_read.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            tx.send(Request {
                command: ReadCommand::ReadU64LE,
                response: response_tx,
                cancel_token: child_token,
            })
            .await
            .expect("broken connection");

            response_rx
                .await
                .expect("broken connection")
                .map(|(n, _)| n)
        }
    }

    pub fn write_all(&self, data: Vec<u8>) -> impl Future<Output = std::io::Result<()>> {
        let tx = self.tx_write.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            tx.send(Request {
                command: WriteCommand::WriteAll(data),
                response: response_tx,
                cancel_token: child_token,
            })
            .await
            .expect("broken connection");

            response_rx.await.expect("broken connection")
        }
    }

    pub fn read_exact(&self, buf: Vec<u8>) -> impl Future<Output = std::io::Result<Vec<u8>>> {
        let tx = self.tx_read.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            tx.send(Request {
                command: ReadCommand::ReadExact(buf),
                response: response_tx,
                cancel_token: child_token,
            })
            .await
            .expect("broken connection");

            response_rx
                .await
                .expect("broken connection")
                .map(|(_, buf)| buf.unwrap())
        }
    }

    async fn process_writes(
        mut writer: OwnedWriteHalf,
        mut rx: Receiver<Request<WriteCommand, ()>>,
    ) {
        loop {
            while let Some(Request {
                command,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                match command {
                    WriteCommand::WriteAll(data) => {
                        // Not cancel safe. Use cancel token to cancel on drop
                        tokio::select! {
                            result = writer.write_all(&data) => {
                                let _ = response.send(result);
                            }
                            _ = cancel_token.cancelled() => {}
                        }
                    }
                }
            }
        }
    }

    async fn process_reads(
        mut reader: OwnedReadHalf,
        mut rx: Receiver<Request<ReadCommand, (u64, Option<Vec<u8>>)>>,
    ) {
        loop {
            while let Some(Request {
                command,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                match command {
                    ReadCommand::ReadBuf(mut buf) => {
                        tokio::select! {
                            result = reader.read_buf(&mut buf) => {
                                let _ = response.send(result.map(|b| (b as u64, Some(buf))));
                            }
                            _ = cancel_token.cancelled() => {}
                        }
                    }

                    ReadCommand::ReadExact(mut buf) => {
                        // Not cancel safe. Use cancel token to cancel on drop
                        tokio::select! {
                            result = reader.read_exact(&mut buf) => {
                                let _ = response.send(result.map(|b| (b as u64, Some(buf))));
                            }
                            _ = cancel_token.cancelled() => {}
                        }
                    }
                    ReadCommand::ReadU64LE => {
                        // Not cancel safe. Use cancel token to cancel on drop
                        tokio::select! {
                            result =  reader.read_u64_le() => {
                                let _ = response.send(result.map(|n| (n, None)));
                            }
                            _ = cancel_token.cancelled() => {}
                        }
                    }
                }
            }
        }
    }
}
