
use crate::universal::gates::{TradeContext, run_gates, LiquidityGate, McapGate, VolatilityGate, PumpFunMigrationGate};
use super::models::{SimInput, SimResult};

pub struct SimConfig {
    pub liq5m: f64,
    pub liq15m: f64,
    pub depth_mult: f64,
    pub mcap_min: f64,
    pub vol_max_pct: f64,
    pub exclude_non_migrated: bool,
}

pub fn execute_simulated(input: &SimInput, ctx: &TradeContext, cfg: &SimConfig) -> SimResult {
    let gates: Vec<Box<dyn crate::universal::gates::Gate>> = vec![
        Box::new(LiquidityGate{min5m:cfg.liq5m, min15m:cfg.liq15m, depth_mult_min:cfg.depth_mult}),
        Box::new(McapGate{min_mcap:cfg.mcap_min}),
        Box::new(VolatilityGate{max_pct:cfg.vol_max_pct}),
        Box::new(PumpFunMigrationGate{exclude_non_migrated:cfg.exclude_non_migrated}),
    ];
    let (ok, reasons) = run_gates(ctx, &gates);
    if !ok {
        let reason = reasons.into_iter().map(|(n,r)| format!("{n}:{r}")).collect::<Vec<_>>().join("|");
        return SimResult{ ts: input.ts, mint: input.mint.clone(), filled: false, reason: Some(reason), exit_ts: None, pnl_usd: 0.0 };
    }
    // TODO fill logic using slippage tolerance and available depth
    SimResult{ ts: input.ts, mint: input.mint.clone(), filled: true, reason: None, exit_ts: None, pnl_usd: 0.0 }
}
