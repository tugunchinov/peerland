use crate::tests::{
    configure_node_static, test_setup, wait_nodes, BrokenUnixTimeProvider, DeterministicRandomizer,
};
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_gossip() {
    test_setup();

    let node_names = [
        "node_1", "node_2", "node_3", "node_4", "node_5", "node_6", "node_7", "node_8", "node_9",
        "node_10",
    ];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        // TODO: use Arc(Mutex) of rngs
        let rng = DeterministicRandomizer::new(seed);
        let mut matrix = turmoil::Builder::new()
            .fail_rate(0.0)
            .tcp_capacity(usize::MAX >> 3)
            .enable_random_order()
            .build_with_rng(Box::new(rng.clone()));

        let (tx, rx) = std::sync::mpsc::channel();

        let broadcast_msg_cnt = 1;
        for (node_idx, node_name) in node_names.iter().enumerate() {
            let node_routine = {
                let tx = tx.clone();

                move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _>>| async move {
                    for i in 0..broadcast_msg_cnt {
                        let msg = format!("{node_idx}:{i}");
                        node.gossip(&msg).await.unwrap();
                        tokio::time::sleep(Duration::from_secs(rand::random::<u64>() % 10)).await;
                    }

                    tracing::warn!("finished spaming");

                    tx.send(node).unwrap();
                }
            };

            matrix.host(
                *node_name,
                configure_node_static::<_, _, LamportClock, _, _>(
                    node_names.iter(),
                    node_idx,
                    rng.clone(),
                    node_routine.clone(),
                ),
            );
        }

        let nodes = wait_nodes(&mut matrix, &node_names, rx);

        let mut storages = Vec::with_capacity(nodes.len());

        for node in nodes {
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
            assert_eq!(storages[i - 1], storages[i]);
        }
    }
}
