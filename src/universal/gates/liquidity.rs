use super::{TradeContext, Gate, GateDecision};

/// Configuration for the liquidity / mcap gate.
///
/// This mirrors the BirdEye liquidity guard thresholds and also keeps
/// the original depth_multiple guard used in the older inline
/// LiquidityGate implementation.
#[derive(Debug, Clone)]
pub struct LiquidityGateConfig {
    pub liq_5m_min_usd: f64,
    pub liq_15m_min_usd: f64,
    pub min_mcap_usd: f64,
    pub depth_mult_min: f64,
}

impl Default for LiquidityGateConfig {
    fn default() -> Self {
        Self {
            liq_5m_min_usd: 15_000.0,
            liq_15m_min_usd: 45_000.0,
            min_mcap_usd: 5_000_000.0,
            // For runtime-from-env usage we default depth_mult_min to 0.0 so it
            // never blocks trades unless explicitly configured (e.g. in sims).
            depth_mult_min: 0.0,
        }
    }
}

impl LiquidityGateConfig {
    /// Build config from env vars, matching the BirdEye patch:
    ///
    /// - LIQ_5M_MIN_USD  (default 15000)
    /// - LIQ_15M_MIN_USD (default 45000)
    /// - MIN_MCAP_USD    (default 5000000)
    ///
    /// depth_mult_min is intentionally *not* read from env here; it is
    /// expected to be set by SimulationConfig when used in the simulator.
    pub fn from_env() -> Self {
        use std::env;

        let mut cfg = Self::default();

        if let Ok(v) = env::var("LIQ_5M_MIN_USD") {
            if let Ok(parsed) = v.parse::<f64>() {
                cfg.liq_5m_min_usd = parsed;
            }
        }

        if let Ok(v) = env::var("LIQ_15M_MIN_USD") {
            if let Ok(parsed) = v.parse::<f64>() {
                cfg.liq_15m_min_usd = parsed;
            }
        }

        if let Ok(v) = env::var("MIN_MCAP_USD") {
            if let Ok(parsed) = v.parse::<f64>() {
                cfg.min_mcap_usd = parsed;
            }
        }

        cfg
    }
}

/// Liquidity gate that evaluates a TradeContext against BirdEye-style thresholds.
///
/// NOTE:
/// - We use 5m/15m volume and mcap here (BirdEye guard semantics).
/// - We also keep the old depth_multiple guard via `depth_mult_min` so
///   SimulationConfig can still enforce a minimum depth multiple.
#[derive(Debug, Clone)]
pub struct LiquidityGate {
    cfg: LiquidityGateConfig,
}

impl LiquidityGate {
    pub fn new(cfg: LiquidityGateConfig) -> Self {
        Self { cfg }
    }

    /// Convenience helper for runtime: build from env.
    pub fn from_env() -> Self {
        Self::new(LiquidityGateConfig::from_env())
    }
}

impl Gate for LiquidityGate {
    fn name(&self) -> &'static str {
        "liquidity"
    }

    fn check(&self, ctx: &TradeContext) -> GateDecision {
        // 1) 5-minute rolling volume guard
        if ctx.window5m_usd < self.cfg.liq_5m_min_usd {
            return GateDecision::Rejected {
                reason: format!(
                    "low_liq_5m {} < {}",
                    ctx.window5m_usd, self.cfg.liq_5m_min_usd
                ),
            };
        }

        // 2) 15-minute rolling volume guard
        if ctx.window15m_usd < self.cfg.liq_15m_min_usd {
            return GateDecision::Rejected {
                reason: format!(
                    "low_liq_15m {} < {}",
                    ctx.window15m_usd, self.cfg.liq_15m_min_usd
                ),
            };
        }

        // 3) Market cap floor (or no_mcap)
        match ctx.est_mcap_usd {
            Some(mcap) => {
                if mcap < self.cfg.min_mcap_usd {
                    return GateDecision::Rejected {
                        reason: format!(
                            "low_mcap {} < {}",
                            mcap, self.cfg.min_mcap_usd
                        ),
                    };
                }
            }
            None => {
                return GateDecision::Rejected {
                    reason: "no_mcap".to_string(),
                };
            }
        }

        // 4) Depth multiple guard (preserved from old LiquidityGate)
        if ctx.depth_multiple < self.cfg.depth_mult_min {
            return GateDecision::Rejected {
                reason: format!(
                    "depth_multiple {} < {}",
                    ctx.depth_multiple, self.cfg.depth_mult_min
                ),
            };
        }

        // Amihud / range efficiency / staleness checks are intentionally omitted
        // for now because TradeContext does not yet carry those fields. Once the
        // Birdeye metrics are threaded through, we can extend this gate without
        // changing its public interface.
        GateDecision::Passed
    }
}
