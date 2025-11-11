use super::WithdrawConfig;
use std::time::{Duration, Instant};

pub struct WithdrawalScheduler { cfg: WithdrawConfig, last: Instant }

impl WithdrawalScheduler {
    pub fn new(cfg: WithdrawConfig) -> Self {
        Self { cfg, last: Instant::now() - Duration::from_secs(cfg.interval_secs) }
    }
    pub fn tick<F>(&mut self, balance_usd: f64, mut do_withdraw: F)
    where F: FnMut(&str, f64) -> anyhow::Result<()> {
        if !self.cfg.enable { return; }
        if balance_usd < self.cfg.threshold_usd { return; }
        if self.last.elapsed().as_secs() < self.cfg.interval_secs { return; }
        if let Ok(()) = do_withdraw(&self.cfg.destination, balance_usd) {
            self.last = Instant::now();
        }
    }
}
