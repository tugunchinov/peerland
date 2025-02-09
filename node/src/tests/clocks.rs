use crate::tests::{
    configure_node_static, test_setup, BrokenUnixTimeProvider, DeterministicRandomizer,
};
use crate::time::LamportClock;
use crate::Node;
use network::discovery::{Discovery, StaticDiscovery};
use network::turmoil;
use rand::Rng;
use std::sync::Arc;

#[test]
fn lt_doesnt_go_backwards() {
    test_setup();

    let node_names = ["node_1", "node_2", "node_3", "node_4", "node_5"];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = DeterministicRandomizer::new(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        let spam_msg_cnt = 100;
        for (node_idx, node_name) in node_names.iter().enumerate() {
            let node_routine = {
                let mut rng = rng.clone();
                let tx = tx.clone();

                move |node: Arc<
                    Node<BrokenUnixTimeProvider<_>, LamportClock, StaticDiscovery<_>, _>,
                >| async move {
                    let known_nodes = node
                        .discovery
                        .list_known_nodes()
                        .into_iter()
                        .collect::<Vec<_>>();

                    for i in 0..spam_msg_cnt {
                        let recipient = &known_nodes[rng.gen::<usize>() % known_nodes.len()];
                        let msg = format!("hello from {node_idx}: {i}");
                        while let Err(e) = node.send_to(recipient, &msg).await {
                            tracing::error!(
                                error = %e,
                                ?recipient,
                                "failed sending message"
                            );
                        }
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
                    node_routine,
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
