use crate::time::{LogicalTimeProvider, Millis, SystemTimeProvider};
use crate::Node;
use network::turmoil;
use network::types::SocketAddr;
use rand::prelude::StdRng;
use rand::Rng;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use sync::SpinLock;

#[cfg(feature = "simulation")]
mod clocks;

#[cfg(feature = "simulation")]
mod broadcast;

#[cfg(feature = "simulation")]
mod communication;

struct TestConfiguration {
    test_rng: StdRng,
    matrix_range: StdRng,
    node_setup_barrier: Arc<tokio::sync::Barrier>,
    node_setup_ready_tx: std::sync::mpsc::Sender<()>,
    node_setup_ready_rx: std::sync::mpsc::Receiver<()>,
}

static LOG_INIT: std::sync::Once = std::sync::Once::new();
static TEST_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

/// Setup function that is only run once, even if called multiple times.
fn test_setup<'a>() -> std::sync::MutexGuard<'a, ()> {
    LOG_INIT.call_once(|| {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .finish(),
        )
        .expect("Configure tracing");
    });

    TEST_MUTEX.lock().unwrap()
}

// TODO: extract mod

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

fn configure_node<
    AsRefStr: AsRef<str>,
    Entropy: Rng + Clone + Send + Sync + 'static,
    LT: LogicalTimeProvider + Send + Sync + 'static,
    Fut: Future<Output = ()>,
    R: FnOnce(Arc<Node<BrokenUnixTimeProvider<Entropy>, LT, Entropy>>) -> Fut + Clone + 'static,
>(
    node_names: &'static [AsRefStr],
    node_idx: usize,
    rng: Entropy,
    node_routine: R,
    barrier: Arc<tokio::sync::Barrier>,
    node_ready: std::sync::mpsc::Sender<()>,
) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
    move || {
        let others = node_names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != node_idx)
            .map(|(_, n)| n)
            .collect::<Vec<_>>();

        let peers = node_names_to_addresses(&others);

        let rng = rng.clone();
        let node_routine = node_routine.clone();
        let node_ready = node_ready.clone();
        let barrier = barrier.clone();
        Box::pin(async move {
            let time_provider = BrokenUnixTimeProvider::new(rng.clone());
            let entropy = rng.clone();

            let node = Node::<_, _, _>::new(
                node_idx as u32,
                (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                time_provider,
                entropy,
                peers.into_iter(),
            )
            .await
            .unwrap();

            node_ready.send(()).unwrap();

            barrier.wait().await;
            node_routine(node).await;

            Ok(())
        })
    }
}

fn node_names_to_addresses<S: AsRef<str>>(node_names: &[S]) -> Vec<SocketAddr> {
    let mut addresses = Vec::with_capacity(256);
    for node_name in node_names {
        addresses.push(SocketAddr::from((
            turmoil::lookup(node_name.as_ref()),
            9000,
        )));
    }

    addresses
}

fn wait_nodes(matrix: &mut turmoil::Sim, node_names: &[&str], timeout_secs: u64) {
    let now = std::time::Instant::now();

    for name in node_names {
        while matrix.is_host_running(*name) {
            let elapsed = now.elapsed().as_secs();
            if elapsed > timeout_secs {
                tracing::info!("Testing too long... Give up.");
                return;
            }
            matrix.run().unwrap();
        }
    }
}
