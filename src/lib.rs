pub fn start() {
    #[cfg(feature = "simulation")]
    node::tests::test();
}
