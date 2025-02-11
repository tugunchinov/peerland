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
fn reliable_broadcast() {
    test_setup();

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

        let broadcast_msg_cnt = 10;
        let barrier = Arc::new(tokio::sync::Barrier::new(NODE_NAMES.len()));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        for (node_idx, node_name) in NODE_NAMES.iter().enumerate() {
            let node_routine = {
                let tx = tx.clone();
                let mut routine_rng = StdRng::seed_from_u64(common_rng.gen());

                move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _>>| async move {
                    for i in 0..broadcast_msg_cnt {
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

                    tracing::warn!("finished spaming");

                    tx.send(node).unwrap();
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

        let nodes = wait_nodes(&mut matrix, &NODE_NAMES, rx);

        let start_ts = std::time::Instant::now();
        let (storage_1, storage_2) = 'out: loop {
            let mut storages = Vec::with_capacity(nodes.len());

            for node in nodes.iter() {
                let storage_guard = node.storage.lock();

                let mut storage = Vec::with_capacity(storage_guard.len());
                for msg in storage_guard.iter() {
                    let msg_str = String::from_utf8(msg.clone()).unwrap();
                    storage.push(msg_str);
                }
                storage.sort();
                storages.push(storage);
            }

            for i in 1..storages.len() {
                if storages[i - 1] != storages[i] {
                    let elapsed = start_ts.elapsed();

                    if elapsed.as_secs() > 30 {
                        let (failed_storage_1, failed_storage_2) =
                            (storages[i - 1].clone(), storages[i].clone());

                        break 'out (failed_storage_1, failed_storage_2);
                    }

                    // Maybe still in progress...
                    break;
                }
            }

            matrix.run().unwrap();
        };

        // Last chance...
        matrix.run().unwrap();

        assert_eq!(storage_1, storage_2);
    }
}
