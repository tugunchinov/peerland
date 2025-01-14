use crate::sync::SpinLock;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millis(pub u128);

impl From<u128> for Millis {
    fn from(val: u128) -> Self {
        Self(val)
    }
}

pub trait SystemTimeProvider: Send + Sync + 'static {
    fn now_millis(&self) -> Millis;
}

pub struct UnixTimeProvider;

impl SystemTimeProvider for UnixTimeProvider {
    fn now_millis(&self) -> Millis {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .into()
    }
}

pub trait LogicalTimeProvider: Send + Sync + 'static {
    type Unit: Ord + Into<crate::proto::message::node_message::Lt>;
    fn new_with_id(id: u32) -> Self;
    fn tick(&self) -> Self::Unit;

    // TODO: add Result
    fn adjust_from_message(&self, message: &crate::proto::message::NodeMessage);
}

pub(crate) struct LamportClock {
    /// Must be unique. Otherwise, there isn't the guarantee about strong monotonicity.
    id: u32,
    counter: AtomicU64,

    #[cfg(debug_assertions)]
    previous_lt: SpinLock<Option<LamportClockUnit>>,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) struct LamportClockUnit(pub(crate) (u64, u32));

impl From<LamportClockUnit> for crate::proto::message::node_message::Lt {
    fn from(value: LamportClockUnit) -> Self {
        use crate::proto::logical_time::LamportClockUnit;
        use crate::proto::message::node_message::Lt;

        Lt::LamportClock(LamportClockUnit {
            lt: value.0 .0,
            proc_id: value.0 .1,
        })
    }
}

// TODO: check if it's correct
impl LogicalTimeProvider for LamportClock {
    type Unit = LamportClockUnit;

    fn new_with_id(id: u32) -> Self {
        Self::new(id)
    }

    fn tick(&self) -> Self::Unit {
        let maybe_previous_lt_lock = if cfg!(debug_assertions) {
            Some(self.previous_lt.lock())
        } else {
            None
        };

        let lt = self.counter.fetch_add(1, Ordering::Acquire);
        let unit = LamportClockUnit((lt, self.id));

        if cfg!(debug_assertions) {
            let mut last_lt = maybe_previous_lt_lock.unwrap();

            if !last_lt.is_none() {
                assert!(
                    last_lt.as_ref().unwrap() < &unit,
                    "logical time reverted: last: {:?}, current: {:?}",
                    *last_lt,
                    &unit,
                );

                *last_lt = Some(unit);
            }
        }

        unit
    }

    fn adjust_from_message(&self, msg: &crate::proto::message::NodeMessage) {
        use crate::proto::logical_time::LamportClockUnit;
        use crate::proto::message::node_message::Lt;

        let mut current = self.counter.load(Ordering::Acquire);
        let Some(Lt::LamportClock(LamportClockUnit { lt, .. })) = msg.lt else {
            return;
        };

        if lt > current {
            loop {
                match self.counter.compare_exchange_weak(
                    current,
                    lt,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        tracing::warn!("timestamp adjusted: {current} -> {lt}");
                        break;
                    }
                    Err(actual) => {
                        current = actual;

                        if current >= lt {
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl LamportClock {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            counter: AtomicU64::new(0),

            #[cfg(debug_assertions)]
            previous_lt: SpinLock::new(None),
        }
    }
}
