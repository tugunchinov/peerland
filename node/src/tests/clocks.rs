use crate::time::BrokenUnixTimeProvider;
use crate::{time, Node};
use network::turmoil;
use rand::prelude::SliceRandom;
use std::cell::RefCell;
use std::future::Future;
use std::mem::transmute;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::mpsc::{Sender, TryRecvError};
use std::time::Duration;
use tokio::sync::SemaphorePermit;

#[test]
fn lt_doesnt_go_backwards() {
    use rand::SeedableRng;

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .expect("Configure tracing");

    let node_names = ["node_1", "node_2", "node_3", "node_4", "node_5"];

    for i in 0..10 {
        let seed = rand::random();

        tracing::info!(%seed, "start {i}th simulation");

        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let boxed_rng = Box::new(rng.clone());
        let mut matrix = turmoil::Builder::new()
            .enable_random_order()
            .build_with_rng(boxed_rng);

        let (tx, rx) = std::sync::mpsc::channel();

        for (i, node_name) in node_names.iter().enumerate() {
            matrix.host(
                *node_name,
                configure_node::<_, _, time::LamportClock>(
                    node_names.to_vec(),
                    i,
                    rng.clone(),
                    100,
                    tx.clone(),
                ),
            );
        }

        let mut nodes_finished = 0;
        while nodes_finished < node_names.len() {
            matrix.run().unwrap();

            match rx.try_recv() {
                Ok(_) => nodes_finished += 1,
                Err(_) => {}
            }
        }

        matrix.run().unwrap();
    }
}

fn configure_node<
    S: AsRef<str>,
    T: rand::Rng + Clone + Send + 'static,
    LT: time::LogicalTimeProvider<Unit = time::LamportClockUnit>,
>(
    nodes_name: Vec<S>,
    node_idx: usize,
    rng: T,
    spam_msg_cnt: u64,
    tx: Sender<()>,
) -> impl Fn() -> Pin<Box<dyn Future<Output = turmoil::Result>>> {
    move || {
        let mut rng = rng.clone();

        let mut other_nodes = Vec::with_capacity(nodes_name.len());
        for (i, node_name) in nodes_name.iter().enumerate() {
            if i != node_idx {
                other_nodes.push((node_name.as_ref().to_string(), 9000));
            }
        }

        let tx = tx.clone();

        Box::pin(async move {
            let time_provider = BrokenUnixTimeProvider::new(rng.clone());

            let node = Node::<_, LT>::new(
                node_idx as u32,
                (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9000),
                time_provider,
            )
            .await
            .unwrap();

            // start spaming
            for i in 0..spam_msg_cnt {
                let msg = format!("hello from {node_idx}: {i}");

                let recipients_cnt = (rand::random::<usize>() % other_nodes.len()).max(1);

                tracing::info!(
                    spam_msg_cnt = ?spam_msg_cnt,
                    recipients_cnt = %recipients_cnt,
                    "starting spaming"
                );

                for _ in 0..recipients_cnt {
                    let recipient = other_nodes.choose(&mut rng).unwrap();

                    if let Err(e) = node.send_message(&msg, recipient).await {
                        tracing::error!(error = %e, "failed sending message. retrying...");
                        //tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                }
            }

            tracing::warn!("finished spaming");

            tx.send(()).unwrap();

            Ok(())
        })
    }
}
