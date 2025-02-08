impl From<crate::proto::time::logical::LamportClockUnit> for crate::time::LamportClockUnit {
    fn from(value: crate::proto::time::logical::LamportClockUnit) -> Self {
        Self((value.lt, value.proc_id))
    }
}

impl From<crate::time::LamportClockUnit> for crate::proto::time::logical::LamportClockUnit {
    fn from(value: crate::time::LamportClockUnit) -> Self {
        Self {
            lt: value.0 .0,
            proc_id: value.0 .1,
        }
    }
}

impl From<crate::time::Millis> for prost_types::Timestamp {
    fn from(value: crate::time::Millis) -> Self {
        Self {
            seconds: (value.0 / 1_000) as i64,
            nanos: ((value.0 % 1_000) * 1_000_000) as i32,
        }
    }
}

impl From<prost_types::Timestamp> for crate::time::Millis {
    fn from(value: prost_types::Timestamp) -> Self {
        Self((value.seconds as u128 * 1000) + (value.nanos as u128 / 1_000_000))
    }
}

impl From<uuid::Uuid> for crate::proto::message::Uuid {
    fn from(value: uuid::Uuid) -> Self {
        crate::proto::message::Uuid {
            value: value.into(),
        }
    }
}

impl TryFrom<crate::proto::message::Uuid> for uuid::Uuid {
    type Error = uuid::Error;

    fn try_from(value: crate::proto::message::Uuid) -> Result<Self, Self::Error> {
        uuid::Uuid::from_slice(&value.value)
    }
}

impl TryFrom<&crate::proto::message::Uuid> for uuid::Uuid {
    type Error = uuid::Error;

    fn try_from(value: &crate::proto::message::Uuid) -> Result<Self, Self::Error> {
        uuid::Uuid::from_slice(&value.value)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let proto_uuid: crate::proto::message::Uuid = uuid.into();

        let uuid_again: uuid::Uuid = proto_uuid.try_into().unwrap();

        assert_eq!(uuid, uuid_again);
    }
}
