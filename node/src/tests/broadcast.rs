use crate::tests::{configure_node_static, test_setup, BrokenUnixTimeProvider};
use crate::time::LamportClock;
use crate::Node;
use network::discovery::StaticDiscovery;
use network::turmoil;
use rand::rngs::StdRng;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_gossip() {
    test_setup();

    use rand::SeedableRng;

    let node_names = ["node_1", "node_2", "node_3", "node_4", "node_5"];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = StdRng::seed_from_u64(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .tcp_capacity(usize::MAX >> 3)
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        let broadcast_msg_cnt = 10;
        for (node_idx, node_name) in node_names.iter().enumerate() {
            let node_routine = {
                let tx = tx.clone();

                move |node: Arc<
                    Node<BrokenUnixTimeProvider<_>, LamportClock, StaticDiscovery<_>, _>,
                >| async move {
                    for i in 0..broadcast_msg_cnt {
                        let msg = format!("hello from {node_idx}: {i}");
                        node.gossip(&msg, 2).await;
                        tokio::time::sleep(Duration::from_secs(rand::random::<u64>() % 30)).await;
                    }

                    tracing::warn!("finished spaming");

                    tx.send(()).unwrap();

                    std::future::pending().await
                }
            };

            matrix.host(
                *node_name,
                configure_node_static::<_, _, LamportClock, _, _>(
                    node_names.to_vec(),
                    node_idx,
                    rng.clone(),
                    node_routine.clone(),
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
