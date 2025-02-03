use crate::tests::{configure_node, BrokenUnixTimeProvider};
use crate::time::LamportClock;
use crate::Node;
use network::turmoil;
use std::sync::Arc;

#[test]
fn lt_doesnt_go_backwards() {
    use rand::SeedableRng;

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .expect("Configure tracing");

    let node_names = ["node_1", "node_2", "node_3"];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        let spam_msg_cnt = 100;
        for (node_idx, node_name) in node_names.iter().enumerate() {
            let node_routine =
                move |node: Arc<Node<BrokenUnixTimeProvider<_>, LamportClock, _, _>>| async move {
                    for i in 0..spam_msg_cnt {
                        let msg = format!("hello from {node_idx}: {i}");
                        node.gossip(msg.clone().into_bytes(), 1).await
                    }

                    tracing::warn!("finished spaming");
                };

            matrix.host(
                *node_name,
                configure_node::<_, _, LamportClock, _, _, _>(
                    node_names.to_vec(),
                    node_idx,
                    rng.clone(),
                    node_routine,
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
