// #[test]
// fn test_gossip() {
//     use rand::SeedableRng;
//
//     tracing::subscriber::set_global_default(
//         tracing_subscriber::fmt()
//             .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
//             .finish(),
//     )
//     .expect("Configure tracing");
//
//     let node_names = ["node_1", "node_2", "node_3", "node_4", "node_5"];
//
//     for i in 0..10 {
//         let seed = rand::random();
//
//         tracing::info!(%seed, "start {i}th simulation");
//
//         let rng = rand::rngs::StdRng::seed_from_u64(seed);
//         let boxed_rng = Box::new(rng.clone());
//         let mut matrix = turmoil::Builder::new()
//             .enable_random_order()
//             .build_with_rng(boxed_rng);
//
//         let (tx, rx) = std::sync::mpsc::channel();
//
//         let node_routine = |node: Node| async move {};
//
//         for (i, node_name) in node_names.iter().enumerate() {
//             matrix.host(
//                 *node_name,
//                 configure_node::<_, _, time::LamportClock>(
//                     node_names.to_vec(),
//                     i,
//                     rng.clone(),
//                     100,
//                     tx.clone(),
//                 ),
//             );
//         }
//
//         let mut nodes_finished = 0;
//         while nodes_finished < node_names.len() {
//             matrix.run().unwrap();
//
//             if rx.try_recv().is_ok() {
//                 nodes_finished += 1;
//             }
//         }
//
//         matrix.run().unwrap();
//     }
// }
