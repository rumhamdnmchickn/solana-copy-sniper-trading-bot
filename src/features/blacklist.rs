use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PositionKey { pub copied_wallet: String, pub mint: String }

#[derive(Clone, Default)]
pub struct PositionRegistry { set: Arc<RwLock<HashSet<PositionKey>>> }

impl PositionRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn mark_open(&self, key: PositionKey) { self.set.write().unwrap().insert(key); }
    pub fn mark_closed(&self, key: &PositionKey) { self.set.write().unwrap().remove(key); }
    pub fn has_open(&self, key: &PositionKey) -> bool { self.set.read().unwrap().contains(key) }
}
