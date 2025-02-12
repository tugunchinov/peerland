use crate::tests::static_network::no_failure::{node_names_to_addresses, test_setup, wait_nodes};
use crate::tests::utils::time::BrokenUnixTimeProvider;
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::Arc;

const NODE_NAMES: [&str; 5] = ["node_1", "node_2", "node_3", "node_4", "node_5"];
const SPAM_MSG_CNT: usize = 100;

#[test]
fn lt_doesnt_go_backwards() {
    let _test_guard = test_setup();

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let mut common_rng = StdRng::seed_from_u64(seed);

        let matrix_rng = Box::new(StdRng::seed_from_u64(common_rng.gen()));
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(matrix_rng);

        let barrier = Arc::new(tokio::sync::Barrier::new(NODE_NAMES.len()));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        for (node_idx, node_name) in NODE_NAMES.iter().enumerate() {
            matrix.host(
                *node_name,
                gossip_node_routine(
                    node_idx,
                    &mut common_rng,
                    Arc::clone(&barrier),
                    ready_tx.clone(),
                ),
            );

            while ready_rx.try_recv().is_err() {
                matrix.run().unwrap();
            }
        }

        wait_nodes(&mut matrix, &NODE_NAMES, None);
    }
}

fn gossip_node_routine(
    node_idx: usize,
    test_rng: &mut StdRng,
    barrier: Arc<tokio::sync::Barrier>,
    node_ready: std::sync::mpsc::Sender<()>,
) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
    let time_seed = test_rng.gen::<u64>();
    let node_seed = test_rng.gen::<u64>();
    let routine_seed = test_rng.gen::<u64>();

    move || {
        let node_ready = node_ready.clone();
        let barrier = barrier.clone();
        Box::pin(async move {
            let others = NODE_NAMES
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != node_idx)
                .map(|(_, n)| n)
                .collect::<Vec<_>>();
            let peers = node_names_to_addresses(&others);

            let time_provider_rng = StdRng::seed_from_u64(time_seed);
            let time_provider = BrokenUnixTimeProvider::new(time_provider_rng);

            tracing::info!("node {node_idx} is starting...");

            let node_rng = StdRng::seed_from_u64(node_seed);
            let node = Node::<_, LamportClock, _>::new(
                node_idx as u32,
                (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                time_provider,
                node_rng,
                peers.into_iter(),
            )
            .await
            .unwrap();

            node_ready.send(()).unwrap();

            tracing::info!("node {node_idx} started.");

            barrier.wait().await;

            let mut routine_rng = StdRng::seed_from_u64(routine_seed);

            for i in 0..SPAM_MSG_CNT {
                let msg = format!("hello from {}: {i}", node.id);
                while let Err(e) = node.gossip(msg.clone()).await {
                    tracing::error!(
                        error = %e,
                        "failed broadcasting message"
                    );
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(
                    routine_rng.gen::<u64>() % 15,
                ))
                .await;
            }

            tracing::warn!("finished spaming");

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            Ok(())
        })
    }
}
