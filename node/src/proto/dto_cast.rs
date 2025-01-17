impl From<crate::proto::lt::LamportClockUnit> for crate::time::LamportClockUnit {
    fn from(value: crate::proto::lt::LamportClockUnit) -> Self {
        Self((value.lt, value.proc_id))
    }
}

impl From<crate::time::LamportClockUnit> for crate::proto::lt::LamportClockUnit {
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
