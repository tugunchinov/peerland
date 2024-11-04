use crate::error::NodeError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
enum MessageKind {
    Ordinary = 0,
    Notify = 1,
}

const MESSAGE_BUF_DEFAULT_CAPACITY: usize = 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Message {
    id: uuid::Uuid,
    kind: MessageKind,
    data: u128,
}

impl Message {
    pub(crate) fn new_ordinary(data: u128) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            kind: MessageKind::Ordinary,
            data,
        }
    }

    pub(crate) fn new_notify() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            kind: MessageKind::Notify,
            data: 0,
        }
    }

    pub(crate) fn is_notify(&self) -> bool {
        self.kind == MessageKind::Notify
    }

    pub(crate) fn pack(&self) -> Result<Vec<u8>, NodeError> {
        let mut buf = Vec::with_capacity(MESSAGE_BUF_DEFAULT_CAPACITY);

        serde::Serialize::serialize(&self, &mut rmp_serde::Serializer::new(&mut buf))?;

        Ok(buf)
    }

    pub(crate) fn unpack(data: &[u8]) -> Result<Self, NodeError> {
        Ok(serde::Deserialize::deserialize(
            &mut rmp_serde::Deserializer::new(data),
        )?)
    }

    pub(crate) const fn max_size() -> usize {
        size_of::<Self>()
    }
}
