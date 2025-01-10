use crate::sync::SpinLock;
use rand::Rng;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

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

// TODO: move to tests
pub(crate) struct BrokenUnixTimeProvider<R: Rng> {
    start_time: SystemTime,
    last_ts: AtomicU64,
    rng: SpinLock<R>,
}

impl<R: Rng + Send + 'static> BrokenUnixTimeProvider<R> {
    pub fn new(rng: R) -> Self {
        let last_ts = AtomicU64::new(0);
        let start_time = SystemTime::now();

        let rng = SpinLock::new(rng);

        Self {
            start_time,
            last_ts,
            rng,
        }
    }
}

impl<R: Rng + Send + 'static> SystemTimeProvider for BrokenUnixTimeProvider<R> {
    fn now_millis(&self) -> Millis {
        let mut rng_guard = self.rng.lock();
        let run_faster = rng_guard.gen_bool(0.5);
        let run_slower = rng_guard.gen_bool(0.5);
        drop(rng_guard);

        let mut now = self.start_time.elapsed().unwrap().as_millis() as u64;

        let mut last_ts = self.last_ts.load(Ordering::Relaxed);
        loop {
            if now < last_ts {
                return (last_ts as u128).into();
            }

            if run_faster {
                now += 100 + (rand::random::<u64>() % 1001); // add from 0.1 to 1 sec
            } else if run_slower {
                let diff = now - last_ts;
                if diff > 0 {
                    let mut rng_guard = self.rng.lock();
                    now = last_ts + (rng_guard.gen::<u64>() % diff);
                }
            }

            match self.last_ts.compare_exchange_weak(
                last_ts,
                now,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => last_ts = actual,
            }
        }

        (now as u128).into()
    }
}

pub trait LogicalTimeProvider: Send + Sync + 'static {
    type Unit: Ord;
    fn new_with_id(id: u32) -> Self;
    fn next_timestamp(&self) -> Self::Unit;
    fn adjust_timestamp(&self, timestamp: Self::Unit);
}

pub(crate) struct LamportClock {
    /// Must be unique. Otherwise, there isn't the guarantee about strong monotonicity.
    id: u32,
    counter: AtomicU64,

    #[cfg(debug_assertions)]
    previous_lt: SpinLock<LamportClockUnit>,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) struct LamportClockUnit(pub(crate) (u64, u32));

// TODO: check if it's correct
impl LogicalTimeProvider for LamportClock {
    type Unit = LamportClockUnit;

    fn new_with_id(id: u32) -> Self {
        Self::new(id)
    }

    fn next_timestamp(&self) -> Self::Unit {
        let maybe_previous_lt_lock = if cfg!(debug_assertions) {
            Some(self.previous_lt.lock())
        } else {
            None
        };

        let lt = self.counter.fetch_add(1, Ordering::Acquire);
        let unit = LamportClockUnit((lt, self.id));

        if cfg!(debug_assertions) {
            let mut last_lt = maybe_previous_lt_lock.unwrap();

            assert!(
                last_lt.deref() < &unit,
                "logical time reverted: last: {:?}, current: {:?}",
                *last_lt,
                &unit,
            );

            *last_lt = unit;
        }

        unit
    }

    fn adjust_timestamp(&self, ts: Self::Unit) {
        let mut current = self.counter.load(Ordering::Acquire);
        let ts_millis = ts.0 .0;

        if ts_millis > current {
            loop {
                match self.counter.compare_exchange_weak(
                    current,
                    ts_millis,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        tracing::warn!("timestamp adjusted: {current} -> {ts_millis}");
                        break;
                    }
                    Err(actual) => {
                        current = actual;

                        if current >= ts_millis {
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
            previous_lt: SpinLock::new(LamportClockUnit::default()),
        }
    }
}
