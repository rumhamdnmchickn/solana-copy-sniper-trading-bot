use crate::universal::gates::{
    TradeContext,
    run_gates,
    LiquidityGate,
    McapGate,
    VolatilityGate,
    PumpFunMigrationGate,
};
use crate::universal::gates::liquidity::LiquidityGateConfig;

/// Configuration for the simulation backend.
/// These thresholds mirror the intent of your gates:
/// - liq5m / liq15m: minimum volume in last 5/15 minutes (USD)
/// - depth_mult: required depth multiple vs. notional
/// - mcap_min: minimum market cap (USD)
/// - vol_max_pct: maximum allowed volatility in the window (percent)
/// - exclude_non_migrated: filter out non-migrated PumpFun tokens
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub liq5m: f64,
    pub liq15m: f64,
    pub depth_mult: f64,
    pub mcap_min: f64,
    pub vol_max_pct: f64,
    pub exclude_non_migrated: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        // Reasonable placeholder defaults; you can later wire these
        // from env or Config.
        Self {
            liq5m: 1_000.0,
            liq15m: 3_000.0,
            depth_mult: 2.0,
            mcap_min: 50_000.0,
            vol_max_pct: 50.0,
            exclude_non_migrated: true,
        }
    }
}

/// What kind of trade is being simulated.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum SimulationAction {
    Buy,
    Sell,
}

/// Structured result of a simulation run.
/// This is meant to be easy to log / serialize and later
/// extended with more P&L / slippage info.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulationResult {
    pub mint: String,
    pub action: SimulationAction,
    /// Did all gates pass?
    pub passed: bool,
    /// If any gate rejected, (gate_name, reason) entries.
    pub gate_reasons: Vec<(String, String)>,
    /// Estimated fill fraction [0.0, 1.0].
    /// For now this is a simple placeholder based on gate pass/fail.
    pub est_fill_pct: f64,
    /// Optional estimated P&L in USD.
    /// You can enrich this later once you have full trade stats.
    pub est_pnl_usd: Option<f64>,
}

/// Trait describing something that can simulate trade execution.
///
/// For Option B, we start with a pure simulation backend (SimBackend),
/// but this trait gives you room to add other backends later if desired.
pub trait ExecutionSimulator {
    fn simulate(&self, ctx: &TradeContext, action: SimulationAction) -> SimulationResult;
}

/// Concrete simulation backend built on top of universal gates.
#[derive(Debug, Clone)]
pub struct SimBackend {
    pub cfg: SimConfig,
}

impl SimBackend {
    pub fn new(cfg: SimConfig) -> Self {
        Self { cfg }
    }

    /// Convenience constructor with default thresholds.
    pub fn default() -> Self {
        Self { cfg: SimConfig::default() }
    }
}

impl ExecutionSimulator for SimBackend {
    fn simulate(&self, ctx: &TradeContext, action: SimulationAction) -> SimulationResult {
        // Build the same set of gates you intend to use in live trading.
        // NOTE: This relies on the gate structs having public fields:
        //   LiquidityGate { min5m, min15m, depth_mult_min }
        //   McapGate { min_mcap }
        //   VolatilityGate { max_pct }
        //   PumpFunMigrationGate { exclude_non_migrated }
        let gates: Vec<Box<dyn crate::universal::gates::Gate>> = vec![
    Box::new(LiquidityGate::new(LiquidityGateConfig {
        liq_5m_min_usd: self.cfg.liq5m,
        liq_15m_min_usd: self.cfg.liq15m,
        min_mcap_usd: self.cfg.mcap_min,
        depth_mult_min: self.cfg.depth_mult,
    })),
    Box::new(McapGate {
        min_mcap: self.cfg.mcap_min,
    }),
    Box::new(VolatilityGate {
        max_pct: self.cfg.vol_max_pct,
    }),
    Box::new(PumpFunMigrationGate {
        exclude_non_migrated: self.cfg.exclude_non_migrated,
    }),
];

        let (ok, reasons) = run_gates(ctx, &gates);

        // For now, we approximate:
        // - if gates passed: 100% fill, unknown P&L.
        // - if gates failed: 0% fill, no P&L.
        //
        // Later, you can:
        // - Model slippage based on est_cost_bps + depth_multiple.
        // - Estimate P&L given entry price / target exit / fees.
        let est_fill_pct = if ok { 1.0 } else { 0.0 };

        SimulationResult {
            mint: ctx.mint.clone(),
            action,
            passed: ok,
            gate_reasons: reasons,
            est_fill_pct,
            est_pnl_usd: None,
        }
    }
}
