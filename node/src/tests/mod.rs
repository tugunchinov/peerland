use crate::time::{Millis, SystemTimeProvider};
use crate::utils::sync::SpinLock;
use crate::{time, Node};
use network::discovery::StaticDiscovery;
use network::turmoil;
use rand::Rng;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

#[cfg(feature = "simulation")]
mod clocks;

#[cfg(feature = "simulation")]
mod broadcast;

use std::sync::Once;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn test_setup() {
    INIT.call_once(|| {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");
    });
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

fn configure_node_static<
    S: AsRef<str>,
    Ent: Rng + Clone + Send + Sync + 'static,
    LT: time::LogicalTimeProvider,
    Fut: Future<Output = ()>,
    R: FnOnce(
            Arc<Node<BrokenUnixTimeProvider<Ent>, LT, StaticDiscovery<(String, u16)>, Ent>>,
        ) -> Fut
        + Clone
        + 'static,
>(
    nodes_name: Vec<S>,
    node_idx: usize,
    rng: Ent,
    node_routine: R,
) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
    move || {
        let mut other_nodes = Vec::with_capacity(nodes_name.len());
        for (i, node_name) in nodes_name.iter().enumerate() {
            if i != node_idx {
                other_nodes.push((node_name.as_ref().to_string(), 9000));
            }
        }

        let rng = rng.clone();
        let node_routine = node_routine.clone();
        Box::pin(async move {
            let time_provider = BrokenUnixTimeProvider::new(rng.clone());
            let discovery = StaticDiscovery::new(other_nodes.clone());
            let entropy = rng.clone();

            let node = Node::<_, LT, _, _>::new(
                node_idx as u32,
                (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                time_provider,
                discovery,
                entropy,
            )
            .await
            .unwrap();

            node_routine(node).await;

            Ok(())
        })
    }
}
