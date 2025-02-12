use crate::tests::{configure_node, test_setup, wait_nodes, BrokenUnixTimeProvider};
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

// const NODE_NAMES: [&str; 10] = [
//     "node_1", "node_2", "node_3", "node_4", "node_5", "node_6", "node_7", "node_8", "node_9",
//     "node_10",
// ];

const NODE_NAMES: [&str; 2] = ["node_1", "node_2"];

#[test]
fn one_node_broadcast() {
    let _test_guard = test_setup();

    for i in 0..1 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let mut common_rng = StdRng::seed_from_u64(seed);

        let matrix_rng = Box::new(StdRng::seed_from_u64(common_rng.gen()));
        let mut matrix = turmoil::Builder::new()
            .fail_rate(0.0)
            .tcp_capacity(usize::MAX >> 3)
            .enable_random_order()
            .build_with_rng(matrix_rng);

        const BROADCAST_MSG_CNT: i32 = 10;

        let barrier = Arc::new(tokio::sync::Barrier::new(NODE_NAMES.len()));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        let node_routine = {
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

                tracing::warn!("finished spaming. start wainting.");

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    let msg_received = node.processed_messages.lock().len();
                    tracing::info!("received {msg_received} messages");

                    if msg_received == 2 * BROADCAST_MSG_CNT as usize {
                        break;
                    }
                }

                tracing::warn!("finished waiting");
            }
        };

        let node_rng = StdRng::seed_from_u64(common_rng.gen());

        matrix.host(
            NODE_NAMES[0],
            configure_node::<_, _, LamportClock, _, _>(
                &NODE_NAMES,
                0,
                node_rng,
                node_routine,
                barrier.clone(),
                ready_tx.clone(),
            ),
        );

        // Wait node to load
        while ready_rx.try_recv().is_err() {
            matrix.run().unwrap();
        }

        let node_routine = {
            let mut routine_rng = StdRng::seed_from_u64(common_rng.gen());

            move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _>>| async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    let msg_received = node.processed_messages.lock().len();
                    tracing::info!("received {msg_received} messages");

                    if msg_received == BROADCAST_MSG_CNT as usize {
                        break;
                    }
                }

                tracing::warn!("finished waiting. start spaming...");

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
            }
        };

        let node_rng = StdRng::seed_from_u64(common_rng.gen());

        matrix.host(
            NODE_NAMES[1],
            configure_node::<_, _, LamportClock, _, _>(
                &NODE_NAMES,
                1,
                node_rng,
                node_routine,
                barrier.clone(),
                ready_tx.clone(),
            ),
        );

        wait_nodes(&mut matrix, &NODE_NAMES, 60);
    }
}
