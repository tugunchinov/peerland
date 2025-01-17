use crate::discovery::Discovery;
use crate::types::ToSocketAddrs;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::RwLock;
// TODO: make it less restrictive?

pub struct StaticDiscovery<T: ToSocketAddrs + Clone + Eq + Hash + Send + Sync + 'static> {
    known_nodes: RwLock<HashSet<T>>,
}

impl<T: ToSocketAddrs + Clone + Eq + Hash + Send + Sync + 'static> StaticDiscovery<T> {
    pub fn new(known_nodes: impl IntoIterator<Item = T>) -> Self {
        Self {
            known_nodes: RwLock::new(known_nodes.into_iter().collect()),
        }
    }

    pub fn add_node(&self, node: T) {
        self.known_nodes
            .write()
            .expect("lock poisoned")
            .insert(node);
    }
}

// TODO: optimize
impl<T: ToSocketAddrs + Clone + Eq + Hash + Send + Sync + 'static> Discovery
    for StaticDiscovery<T>
{
    fn list_known_nodes(&self) -> impl IntoIterator<Item = impl ToSocketAddrs> {
        let guard = self.known_nodes.read().expect("lock poisoned");
        guard.iter().map(|v| v.clone()).collect::<Vec<T>>()
    }

    fn get_random_nodes(
        &self,
        cnt: usize,
        mut entropy: impl Rng,
    ) -> impl IntoIterator<Item = impl ToSocketAddrs> {
        let guard = self.known_nodes.read().expect("lock poisoned");
        let mut list = guard.iter().map(|v| v.clone()).collect::<Vec<T>>();
        list.shuffle(&mut entropy);
        list.into_iter().take(cnt)
    }
}
