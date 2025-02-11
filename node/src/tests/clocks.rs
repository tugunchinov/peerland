use crate::tests::{
    configure_node_static, node_names_to_addresses, setup_hosts, setup_nodes, test_setup,
    wait_nodes, BrokenUnixTimeProvider, DeterministicRandomizer,
};
use crate::time::LamportClock;
use crate::{time, Node};
use network::turmoil;
use rand::Rng;
use std::sync::Arc;

const NODE_NAMES: [&str; 5] = ["node_1", "node_2", "node_3", "node_4", "node_5"];

#[test]
fn lt_doesnt_go_backwards() {
    test_setup();

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = DeterministicRandomizer::new(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();
        matrix.host("setup", setup_nodes(&NODE_NAMES, rng.clone(), tx));

        let nodes = loop {
            matrix.run().unwrap();

            let maybe_nodes = rx.try_recv();
            if let Ok(nodes) = maybe_nodes {
                break nodes;
            }
        };

        let (tx, rx) = std::sync::mpsc::channel();

        let spam_msg_cnt = 100;
        for (i, node) in nodes.enumerate() {
            matrix.host(
                NODE_NAMES[i],
                configure_node_static::<_, _, LamportClock, _, _>(
                    node_names.iter(),
                    node_idx,
                    rng.clone(),
                    node_routine,
                ),
            );

            // Connect nodes
            for _ in 0..1_000_000 {
                matrix.run().unwrap();
            }
        }

        panic!("AAAA");

        let _ = wait_nodes(&mut matrix, &node_names, rx);
    }
}

async fn start_sending_messages<
    ST: time::SystemTimeProvider,
    LT: time::LogicalTimeProvider,
    R: Rng,
>(
    node: Arc<Node<ST, LT, R>>,
    msg_cnt: usize,
    mut rng: R,
    ready: std::sync::mpsc::Sender<Arc<Node<ST, LT, R>>>,
) {
    let known_nodes = node_names_to_addresses(NODE_NAMES.iter());

    for i in 0..msg_cnt {
        let recipient = &known_nodes[rng.gen::<usize>() % known_nodes.len()];
        let msg = format!("hello from {node_idx}: {i}");
        while let Err(e) = node.send_to(*recipient, &msg).await {
            tracing::error!(
                error = %e,
                ?recipient,
                "failed sending message"
            );
        }
    }

    tracing::warn!("finished spaming");

    ready.send(node).unwrap();
}
