mod static_discovery;

use rand::Rng;
pub use static_discovery::StaticDiscovery;

use crate::types::ToSocketAddrs;

pub trait Discovery: Send + Sync + 'static {
    fn list_known_nodes(&self) -> impl IntoIterator<Item = impl ToSocketAddrs + Send>;
    fn get_random_nodes(
        &self,
        cnt: usize,
        entropy: impl Rng,
    ) -> impl IntoIterator<Item = impl ToSocketAddrs + Send>;
}
