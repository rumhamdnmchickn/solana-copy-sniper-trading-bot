
use std::collections::HashSet;

pub struct Blacklist {
    set: HashSet<String>,
}

impl Blacklist {
    pub fn from_csv(csv: &str) -> Self {
        let set = csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        Self{ set }
    }
    pub fn contains(&self, mint: &str) -> bool { self.set.contains(mint) }
}
