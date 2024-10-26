pub async fn start() {
    let node_1_address = "0.0.0.0:9875";
    let node_2_address = "0.0.0.0:9876";
    let node_3_address = "0.0.0.0:9878";

    let node_1 = node::Node::new(node_1_address)
        .await
        .expect("TODO: make error");
    let node_2 = node::Node::new(node_2_address)
        .await
        .expect("TODO: make error");
    let _node_3 = node::Node::new(node_3_address)
        .await
        .expect("TODO: make error");

    let t1 = tokio::spawn(async move {
        node_1
            .send_message("Hello, Node 2!".to_string(), node_2_address)
            .await
            .expect("TODO: make error");

        let msg = node_1.recv_message().await.expect("TODO: make error");

        println!("NODE 1: received {msg}");
    });

    let t2 = tokio::spawn(async move {
        node_2
            .send_message("Hello, Node 1!".to_string(), node_1_address)
            .await
            .expect("TODO: make error");

        let msg = node_2.recv_message().await.expect("TODO: make error");

        println!("NODE 2: received {msg}");
    });

    t1.await.unwrap();
    t2.await.unwrap();
}
