use crate::error::NodeError;
use crate::{time, Node};
use network::discovery::Discovery;
use network::types::ToSocketAddrs;
use rand::Rng;

impl<
        ST: time::SystemTimeProvider,
        LT: time::LogicalTimeProvider,
        D: Discovery,
        R: Rng + Send + Sync + 'static + Clone,
    > Node<ST, LT, D, R>
{
    pub async fn send_to<B: AsRef<[u8]>>(
        &self,
        to: impl ToSocketAddrs,
        msg: B,
    ) -> Result<(), NodeError> {
        use crate::communication::proto::message::*;

        let serialized = self.create_serialized_node_message(
            msg,
            MessageKind::Addressed(addressed::MessageType::Ordinary.into()),
        );

        self.send_serialized_message(&serialized, to).await
    }
}
