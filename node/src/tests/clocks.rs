use crate::tests::{configure_node, test_setup, wait_nodes, BrokenUnixTimeProvider};
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

const NODE_NAMES: [&str; 5] = ["node_1", "node_2", "node_3", "node_4", "node_5"];

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

        let spam_msg_cnt = 100;

        for (node_idx, node_name) in NODE_NAMES.iter().enumerate() {
            let node_routine = {
                let mut routine_rng = StdRng::seed_from_u64(common_rng.gen());

                move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _>>| async move {
                    for i in 0..spam_msg_cnt {
                        let msg = format!("hello from {}: {i}", node.id);
                        while let Err(e) = node.gossip(msg.clone()).await {
                            tracing::error!(
                                error = %e,
                                "failed gossiping message"
                            );
                        }

                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            routine_rng.gen::<u64>() % 15,
                        ))
                        .await;
                    }

                    tracing::warn!("finished spaming");
                }
            };

            let node_rng = StdRng::seed_from_u64(common_rng.gen());

            matrix.host(
                *node_name,
                configure_node::<_, _, LamportClock, _, _>(
                    &NODE_NAMES,
                    node_idx,
                    node_rng,
                    node_routine,
                    barrier.clone(),
                    ready_tx.clone(),
                ),
            );

            while ready_rx.try_recv().is_err() {
                matrix.run().unwrap();
            }
        }

        wait_nodes(&mut matrix, &NODE_NAMES, 30);
    }
}
