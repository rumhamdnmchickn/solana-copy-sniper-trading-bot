use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::liquidity_guard::TokenMetrics;

/// Very simple in-memory cache keyed by mint.
/// In real bot code this would be fed by Birdeye WS or REST snapshots.
#[derive(Debug, Default)]
pub struct MetricsStore {
    pub map: HashMap<String, TokenMetrics>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    /// Insert/refresh metrics for a token.
    pub fn upsert(&mut self, metrics: TokenMetrics) {
        self.map.insert(metrics.mint.clone(), metrics);
    }

    /// Get latest metrics if present.
    pub fn get(&self, mint: &str) -> Option<&TokenMetrics> {
        self.map.get(mint)
    }

    /// Helper for tests / dry run: create dummy metrics
    pub fn inject_dummy(
        &mut self,
        mint: &str,
        vol_5m_usd: f64,
        vol_15m_usd: f64,
        marketcap_usd: f64,
        amihud_5m: f64,
        range_eff_5m: f64,
    ) {
        let m = TokenMetrics {
            mint: mint.to_string(),
            vol_5m_usd,
            vol_15m_usd,
            liq_usd: Some(vol_5m_usd), // placeholder depth proxy
            marketcap_usd: Some(marketcap_usd),
            amihud_5m: Some(amihud_5m),
            range_eff_5m: Some(range_eff_5m),
            last_update: Utc::now(),
        };
        self.upsert(m);
    }
}
