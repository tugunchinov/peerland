use crate::discovery::Discovery;
use crate::types::SocketAddr;
use rand::seq::SliceRandom;
use rand::Rng;
use std::ops::DerefMut;
use std::sync::RwLock;
use sync::SpinLock;

pub struct StaticDiscovery<R: Rng + Send + 'static> {
    known_nodes: RwLock<Vec<SocketAddr>>,
    // TODO: ???
    entropy: SpinLock<R>,
}

impl<R: Rng + Send + 'static> StaticDiscovery<R> {
    pub fn new(known_nodes: impl IntoIterator<Item = SocketAddr>, entropy: R) -> Self {
        Self {
            known_nodes: RwLock::new(known_nodes.into_iter().collect()),
            entropy: SpinLock::new(entropy),
        }
    }

    pub fn add_node(&self, node: SocketAddr) {
        self.known_nodes.write().expect("lock poisoned").push(node);
    }
}

// TODO: optimize
impl<R: Rng + Send + 'static> Discovery for StaticDiscovery<R> {
    fn list_known_nodes(&self) -> impl IntoIterator<Item = SocketAddr> {
        let guard = self.known_nodes.read().expect("lock poisoned");
        guard.iter().cloned().collect::<Vec<_>>()
    }

    fn get_random_nodes(&self, cnt: usize) -> impl IntoIterator<Item = SocketAddr> {
        // TODO: better
        let guard = self.known_nodes.read().expect("lock poisoned");
        let mut list = guard.iter().cloned().collect::<Vec<_>>();
        list.shuffle(self.entropy.lock().deref_mut());
        list.into_iter().take(cnt)
    }
}
