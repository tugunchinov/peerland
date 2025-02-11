use crate::time::{LogicalTimeProvider, Millis, SystemTimeProvider};
use crate::Node;
use network::turmoil;
use network::types::SocketAddr;
use rand::Rng;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::time::SystemTime;
use sync::SpinLock;

#[cfg(feature = "simulation")]
mod clocks;

#[cfg(feature = "simulation")]
mod broadcast;

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

fn wait_nodes<ST: SystemTimeProvider, LT: LogicalTimeProvider, R: Rng>(
    matrix: &mut turmoil::Sim,
    node_names: &[&str],
    rx: std::sync::mpsc::Receiver<Arc<Node<ST, LT, R>>>,
) -> Vec<Arc<Node<ST, LT, R>>> {
    let expected_count = /* connections + accept */ node_names.len() + /* here from closure */ 1;

    let mut nodes = Vec::with_capacity(node_names.len());
    while nodes.len() < node_names.len() {
        matrix.run().unwrap();

        while let Ok(node) = rx.try_recv() {
            nodes.push(node);
        }
    }

    let now = std::time::Instant::now();

    // make sure everyone has finished
    loop {
        let mut done = true;

        for node in nodes.iter() {
            // 1 listening + 1 here
            if Arc::strong_count(node) > expected_count {
                println!("{}", Arc::strong_count(node));
                done = false;
                break;
            }
        }

        if done {
            break;
        }

        matrix.run().unwrap();

        let elapsed = now.elapsed().as_secs();
        if elapsed > 20 {
            tracing::info!("Testing too long... Give up.");
            break;
        }
    }

    nodes
}
