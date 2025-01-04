use crate::sync::SpinLock;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

pub trait SystemTimeProvider: Send + Sync + 'static {
    fn now_millis(&self) -> u128;
}

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
    fn now_millis(&self) -> u128 {
        let mut rng_guard = self.rng.lock();
        let run_faster = rng_guard.gen_bool(0.5);
        let run_slower = rng_guard.gen_bool(0.5);
        drop(rng_guard);

        let mut now = self.start_time.elapsed().unwrap().as_millis() as u64;

        let mut last_ts = self.last_ts.load(Ordering::Relaxed);
        loop {
            if now < last_ts {
                return last_ts as u128;
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

        now as u128
    }
}

pub trait LogicalTimeProvider: Send + Sync + 'static {
    type Unit: Ord;
    fn next_timestamp(&self) -> Self::Unit;
}

pub(crate) struct LamportClock {
    /// Must be unique. Otherwise, there isn't the guarantee about strong monotonicity.
    id: u8,
    counter: AtomicU64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) struct LamportClockUnit((u64, u8));

// TODO: check if it's correct
impl LogicalTimeProvider for LamportClock {
    type Unit = LamportClockUnit;

    fn next_timestamp(&self) -> Self::Unit {
        let lt = self.counter.fetch_add(1, Ordering::Acquire);
        LamportClockUnit((lt, self.id))
    }
}

impl LamportClock {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            counter: AtomicU64::new(0),
        }
    }

    pub fn adjust_timestamp(&self, ts: u64) {
        let mut current = self.counter.load(Ordering::Acquire);

        if ts > current {
            loop {
                match self.counter.compare_exchange_weak(
                    current,
                    ts,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(actual) => {
                        current = actual;

                        if current >= ts {
                            break;
                        }
                    }
                }
            }
        }
    }
}
