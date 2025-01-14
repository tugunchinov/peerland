use crate::sync::SpinLock;
use crate::time::{Millis, SystemTimeProvider};
use crate::{time, Node};
use network::turmoil;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

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

#[test]
fn lt_doesnt_go_backwards() {
    use rand::SeedableRng;

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .expect("Configure tracing");

    let node_names = ["node_1", "node_2", "node_3", "node_4", "node_5"];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        for (i, node_name) in node_names.iter().enumerate() {
            matrix.host(
                *node_name,
                configure_node::<_, _, time::LamportClock>(
                    node_names.to_vec(),
                    i,
                    rng.clone(),
                    100,
                    tx.clone(),
                ),
            );
        }

        let mut nodes_finished = 0;
        while nodes_finished < node_names.len() {
            matrix.run().unwrap();

            if rx.try_recv().is_ok() {
                nodes_finished += 1;
            }
        }

        matrix.run().unwrap();
    }
}

fn configure_node<
    S: AsRef<str>,
    T: rand::Rng + Clone + Send + 'static,
    LT: time::LogicalTimeProvider<Unit = time::LamportClockUnit>,
>(
    nodes_name: Vec<S>,
    node_idx: usize,
    rng: T,
    spam_msg_cnt: u64,
    tx: Sender<()>,
) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
    move || {
        let mut rng = rng.clone();

        let mut other_nodes = Vec::with_capacity(nodes_name.len());
        for (i, node_name) in nodes_name.iter().enumerate() {
            if i != node_idx {
                other_nodes.push((node_name.as_ref().to_string(), 9000));
            }
        }

        let tx = tx.clone();

        Box::pin(async move {
            let time_provider = BrokenUnixTimeProvider::new(rng.clone());

            let node = Node::<_, LT>::new(
                node_idx as u32,
                (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                time_provider,
            )
            .await
            .unwrap();

            // start spaming
            for i in 0..spam_msg_cnt {
                let msg = format!("hello from {node_idx}: {i}");

                let recipients_cnt = (rand::random::<usize>() % other_nodes.len()).max(1);

                tracing::info!(
                    spam_msg_cnt = ?spam_msg_cnt,
                    recipients_cnt = %recipients_cnt,
                    "starting spaming"
                );

                for _ in 0..recipients_cnt {
                    let recipient = other_nodes.choose(&mut rng).unwrap();

                    if let Err(e) = node.send_message(&msg, recipient).await {
                        tracing::error!(error = %e, "failed sending message. retrying...");
                        //tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                }
            }

            tracing::warn!("finished spaming");

            tx.send(()).unwrap();

            Ok(())
        })
    }
}
