use crate::time::{Millis, SystemTimeProvider};
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use sync::SpinLock;

pub(crate) struct BrokenUnixTimeProvider<R: Rng> {
    start_time: SystemTime,
    last_ts: AtomicU64,
    rng: SpinLock<R>,
}

impl<R: Rng + Send + 'static> BrokenUnixTimeProvider<R> {
    pub fn new(rng: R) -> Self {
        let last_ts = AtomicU64::new(0);
        let start_time = SystemTime::now();

        Self {
            start_time,
            last_ts,
            // bruh...
            rng: SpinLock::new(rng),
        }
    }
}

impl<R: Rng + Sync + Send + 'static> SystemTimeProvider for BrokenUnixTimeProvider<R> {
    fn now_millis(&self) -> Millis {
        let run_faster = self.rng.lock().gen_bool(0.5);
        let run_slower = self.rng.lock().gen_bool(0.5);

        let mut now = self.start_time.elapsed().unwrap().as_millis() as u64;

        let last_ts = self.last_ts.load(Ordering::Relaxed);
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

        self.last_ts.store(now, Ordering::Relaxed);

        (now as u128).into()
    }
}
