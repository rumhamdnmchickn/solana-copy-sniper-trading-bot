use super::{GateOutcome, GateConfig};

pub fn check_all(
    cfg: &GateConfig,
    _ctx: &impl MarketData,
    candidate: &impl CandidateInfo,
) -> GateOutcome {
    if let Some(min5) = cfg.liq_5m_min_usd {
        if candidate.rolling_5m_usd_vol() < min5 { return GateOutcome::Fail("liq_5m"); }
    }
    if let Some(min15) = cfg.liq_15m_min_usd {
        if candidate.rolling_15m_usd_vol() < min15 { return GateOutcome::Fail("liq_15m"); }
    }
    if let Some(mcap_min) = cfg.mcap_min_usd {
        if candidate.mcap_usd() < mcap_min { return GateOutcome::Fail("mcap"); }
    }
    if let Some(vol_max_pp) = cfg.vol_max_pp {
        if candidate.volatility_pp() > vol_max_pp { return GateOutcome::Fail("vol"); }
    }
    if cfg.block_non_migrated_pumpfun && candidate.is_non_migrated_pumpfun() {
        return GateOutcome::Fail("pumpfun_non_migrated");
    }
    GateOutcome::Pass
}

pub trait MarketData {}
pub trait CandidateInfo {
    fn rolling_5m_usd_vol(&self) -> f64;
    fn rolling_15m_usd_vol(&self) -> f64;
    fn mcap_usd(&self) -> f64;
    fn volatility_pp(&self) -> f64;        // percentage points, 0..100
    fn is_non_migrated_pumpfun(&self) -> bool;
}
