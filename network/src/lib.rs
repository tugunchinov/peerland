trait _Peer {
    fn send_message(&self) -> Result<(), impl std::error::Error>;
}
