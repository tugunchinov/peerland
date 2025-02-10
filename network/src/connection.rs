use crate::types::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    SocketAddr, TcpStream,
};
use std::future::Future;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

struct Request<Buf, Res> {
    buf: Buf,
    response: oneshot::Sender<std::io::Result<Res>>,
    cancel_token: tokio_util::sync::CancellationToken,
}

// TODO: use bounded channels
pub struct Connection {
    tx_write: UnboundedSender<Request<&'static [u8], ()>>,
    tx_read: UnboundedSender<Request<&'static mut [u8], usize>>,
}

impl Connection {
    async fn establish(with: &SocketAddr) -> Result<Arc<Self>, std::io::Error> {
        tracing::info!(peer = %with, "trying to establish a connection");

        let connection = TcpStream::connect(with).await?;

        let (reader, writer) = connection.into_split();

        let (tx_write, rx_write) = unbounded_channel();
        tokio::spawn(Self::process_writes(writer, *with, rx_write));

        let (tx_read, rx_read) = unbounded_channel();
        tokio::spawn(Self::process_reads(reader, *with, rx_read));

        tracing::info!(peer = %with, "connection established");

        Ok(Arc::new(Self { tx_write, tx_read }))
    }

    async fn read_u64_le(&self) -> std::io::Result<u64> {
        let mut buf = [0u8; 8];
        let bytes_read = self.read_buf(&mut buf).await?;

        if bytes_read != 8 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        Ok(u64::from_le_bytes(buf))
    }

    fn write_all<'a>(&self, data: &'a [u8]) -> impl Future<Output = std::io::Result<()>> + 'a {
        let tx = self.tx_write.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            // SAFETY: ???TODO
            let data: &'static [u8] = unsafe { std::mem::transmute(data) };

            tx.send(Request {
                buf: data,
                response: response_tx,
                cancel_token: child_token,
            })
            .expect("broken connection");

            response_rx.await.expect("broken connection")
        }
    }

    fn read_buf<'a>(&self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<usize>> + 'a {
        let tx = self.tx_read.clone();

        async move {
            let cancel_token = tokio_util::sync::CancellationToken::new();
            let child_token = cancel_token.child_token();
            let _drop_guard = cancel_token.drop_guard();

            let (response_tx, response_rx) = oneshot::channel();

            // SAFETY: ???TODO
            let buf: &'static mut [u8] = unsafe { std::mem::transmute(buf) };

            tx.send(Request {
                buf,
                response: response_tx,
                cancel_token: child_token,
            })
            .expect("broken connection");

            response_rx.await.expect("broken connection")
        }
    }

    async fn process_writes(
        mut writer: OwnedWriteHalf,
        address: SocketAddr,
        mut rx: UnboundedReceiver<Request<&'static [u8], ()>>,
    ) {
        loop {
            while let Some(Request {
                buf,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                tokio::select! {
                    result = writer.write_all(buf) => {
                        let _ = response.send(result);
                    }
                    _ = cancel_token.cancelled() => {}
                }
            }

            tracing::warn!(peer = %address, "connection closed");
        }
    }

    async fn process_reads(
        mut reader: OwnedReadHalf,
        address: SocketAddr,
        mut rx: UnboundedReceiver<Request<&'static mut [u8], usize>>,
    ) {
        loop {
            while let Some(Request {
                mut buf,
                response,
                cancel_token,
            }) = rx.recv().await
            {
                tokio::select! {
                    result = reader.read_buf(&mut buf) => {
                        let _ = response.send(result);
                    }
                    _ = cancel_token.cancelled() => {}
                }
            }

            tracing::warn!(peer = %address, "connection closed");
        }
    }
}
