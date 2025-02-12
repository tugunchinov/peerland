use crate::tests::{configure_node, test_setup, wait_nodes, BrokenUnixTimeProvider};
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

const NODE_NAMES: [&str; 10] = [
    "node_1", "node_2", "node_3", "node_4", "node_5", "node_6", "node_7", "node_8", "node_9",
    "node_10",
];

const BROADCAST_MSG_CNT: usize = 10;
const EXPECTED_MESSAGE_CNT: usize = BROADCAST_MSG_CNT * NODE_NAMES.len();

#[test]
fn reliable_broadcast() {
    let _test_guard = test_setup();

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let mut common_rng = StdRng::seed_from_u64(seed);

        let matrix_rng = Box::new(StdRng::seed_from_u64(common_rng.gen()));
        let mut matrix = turmoil::Builder::new()
            .fail_rate(0.0)
            .tcp_capacity(usize::MAX >> 3)
            .enable_random_order()
            .build_with_rng(matrix_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        let barrier = Arc::new(tokio::sync::Barrier::new(NODE_NAMES.len()));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        for (node_idx, node_name) in NODE_NAMES.iter().enumerate() {
            let node_routine = {
                let tx = tx.clone();
                let mut routine_rng = StdRng::seed_from_u64(common_rng.gen());

                move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _>>| async move {
                    for i in 0..BROADCAST_MSG_CNT {
                        let msg = format!("hello from {}: {i}", node.id);
                        while let Err(e) = node.broadcast_reliably(msg.clone()).await {
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

                    tracing::warn!("finished spaming. waiting...");

                    loop {
                        let received_msg_cnt = node.storage.lock().len();
                        if received_msg_cnt == EXPECTED_MESSAGE_CNT {
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }

                    tx.send(node.storage.lock().clone()).unwrap();
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

        wait_nodes(&mut matrix, &NODE_NAMES, 120);

        let mut storages = Vec::with_capacity(NODE_NAMES.len());
        for _ in NODE_NAMES {
            let mut storage = rx.try_recv().unwrap();
            storage.sort();
            storages.push(storage);
        }

        for i in 1..storages.len() {
            assert_eq!(storages[i - 1], storages[i]);
        }
    }
}
