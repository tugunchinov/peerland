mod static_discovery;

pub use static_discovery::StaticDiscovery;

use crate::types::SocketAddr;

pub trait Discovery {
    // TODO: remove Debug (and Clone?)
    fn list_known_nodes(&self) -> impl IntoIterator<Item = SocketAddr>;
    fn get_random_nodes(&self, cnt: usize) -> impl IntoIterator<Item = SocketAddr>;
}
