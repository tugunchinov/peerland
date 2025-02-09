use crate::tests::{
    configure_node_static, test_setup, BrokenUnixTimeProvider, DeterministicRandomizer,
};
use crate::time::LamportClock;
use crate::Node;
use network::discovery::StaticDiscovery;
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
            .tcp_capacity(usize::MAX >> 3)
            .enable_random_order()
            .build_with_rng(Box::new(rng.clone()));

        let (tx, rx) = std::sync::mpsc::channel();

        let broadcast_msg_cnt = 10;
        for (node_idx, node_name) in node_names.iter().enumerate() {
            let node_routine = {
                let tx = tx.clone();

                move |node: Arc<
                    Node<BrokenUnixTimeProvider<_>, LamportClock, StaticDiscovery<_>, _>,
                >| async move {
                    for i in 0..broadcast_msg_cnt {
                        let msg = format!("{node_idx}:{i}");
                        node.gossip(&msg).await;
                        tokio::time::sleep(Duration::from_secs(rand::random::<u64>() % 30)).await;
                    }

                    tracing::warn!("finished spaming");

                    tx.send(node.storage.lock().await.clone()).unwrap();

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

        let mut storages = Vec::with_capacity(node_names.len());

        let mut nodes_finished = 0;
        while nodes_finished < node_names.len() {
            matrix.run().unwrap();

            if let Ok(storage) = rx.try_recv() {
                storages.push(storage);
                nodes_finished += 1;
            }
        }

        // TODO: ???
        for _ in 0..1_000 {
            matrix.run().unwrap();
        }

        let mut storages_str = Vec::with_capacity(storages.len());

        for storage in storages {
            let mut storage_str = Vec::with_capacity(storage.len());
            for msg in storage {
                let msg_str = String::from_utf8(msg).unwrap();
                storage_str.push(msg_str);
            }
            storage_str.sort();
            storages_str.push(storage_str);
        }

        for s in storages_str.iter() {
            println!("len = {}", s.len());
        }

        for i in 1..storages_str.len() {
            assert_eq!(storages_str[i - 1], storages_str[i]);
        }
    }
}
