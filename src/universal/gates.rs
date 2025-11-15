
use serde::{Deserialize, Serialize};

pub mod liquidity;
pub use liquidity::LiquidityGate;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeContext {
    pub mint: String,
    pub target_wallet: String,
    pub price_usd: f64,
    pub est_cost_bps: f64,
    pub window5m_usd: f64,
    pub window15m_usd: f64,
    pub depth_multiple: f64,
    pub est_mcap_usd: Option<f64>,
    pub window_vol_pct: f64,
    pub is_pumpfun: bool,
    pub pumpfun_migrated: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GateDecision {
    Passed,
    Rejected { reason: String },
}

pub trait Gate: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, ctx: &TradeContext) -> GateDecision;
}


pub struct McapGate { pub min_mcap: f64 }
pub struct VolatilityGate { pub max_pct: f64 }
pub struct PumpFunMigrationGate { pub exclude_non_migrated: bool }

impl Gate for McapGate {
    fn name(&self) -> &'static str { "McapGate" }
    fn check(&self, ctx: &TradeContext) -> GateDecision {
        match ctx.est_mcap_usd {
            Some(v) if v >= self.min_mcap => GateDecision::Passed,
            Some(v) => GateDecision::Rejected{reason: format!("mcap_usd {} < {}", v, self.min_mcap)},
            None => GateDecision::Rejected{reason: "mcap_usd missing".to_string()},
        }
    }
}

impl Gate for VolatilityGate {
    fn name(&self) -> &'static str { "VolatilityGate" }
    fn check(&self, ctx: &TradeContext) -> GateDecision {
        if ctx.window_vol_pct > self.max_pct {
            return GateDecision::Rejected{reason: format!("vol_pct {} > {}", ctx.window_vol_pct, self.max_pct)};
        }
        GateDecision::Passed
    }
}

impl Gate for PumpFunMigrationGate {
    fn name(&self) -> &'static str { "PumpFunMigrationGate" }
    fn check(&self, ctx: &TradeContext) -> GateDecision {
        if self.exclude_non_migrated && ctx.is_pumpfun {
            if let Some(m) = ctx.pumpfun_migrated {
                if !m { return GateDecision::Rejected{reason: "pumpfun_non_migrated".into()}; }
            } else {
                return GateDecision::Rejected{reason: "pumpfun_migration_unknown".into()};
            }
        }
        GateDecision::Passed
    }
}

pub fn run_gates(ctx: &TradeContext, gates: &[Box<dyn Gate>]) -> (bool, Vec<(String, String)>) {
    let mut reasons = Vec::new();
    for g in gates {
        match g.check(ctx) {
            GateDecision::Passed => {}
            GateDecision::Rejected{reason} => {
                reasons.push((g.name().into(), reason));
                return (false, reasons);
            }
        }
    }
    (true, reasons)
}
