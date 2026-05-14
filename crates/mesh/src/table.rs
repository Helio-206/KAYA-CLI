use std::collections::HashMap;

use kaya_shared::now_millis;

use crate::route::RouteEntry;

#[derive(Debug, Clone)]
pub struct RoutingTable {
    expiry_ms: u64,
    routes: HashMap<String, RouteEntry>,
}

impl RoutingTable {
    pub fn new(expiry_ms: u64) -> Self {
        Self {
            expiry_ms,
            routes: HashMap::new(),
        }
    }

    pub fn upsert(&mut self, mut entry: RouteEntry) {
        let now = now_millis();
        entry.last_seen = now;
        entry.expires_at = now.saturating_add(self.expiry_ms);
        entry.recalculate_score(now);
        match self.routes.get(&entry.destination_node) {
            Some(current) if current.score > entry.score => {}
            _ => {
                self.routes.insert(entry.destination_node.clone(), entry);
            }
        }
    }

    pub fn best_route(&self, target: &str) -> Option<&RouteEntry> {
        self.routes.get(target).or_else(|| {
            self.routes.values().find(|entry| {
                entry
                    .destination_callsign
                    .as_deref()
                    .map(|callsign| callsign.eq_ignore_ascii_case(target))
                    .unwrap_or(false)
            })
        })
    }

    pub fn expire(&mut self, now: u64) -> Vec<RouteEntry> {
        let expired_keys: Vec<_> = self
            .routes
            .iter()
            .filter(|(_, route)| route.is_expired(now))
            .map(|(key, _)| key.clone())
            .collect();
        expired_keys
            .into_iter()
            .filter_map(|key| self.routes.remove(&key))
            .collect()
    }

    pub fn clear(&mut self) {
        self.routes.clear();
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    pub fn entries(&self) -> Vec<RouteEntry> {
        let mut entries: Vec<_> = self.routes.values().cloned().collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.score));
        entries
    }
}
