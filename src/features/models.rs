use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome { Pass, Fail(&'static str) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    pub liq_5m_min_usd: Option<f64>,
    pub liq_15m_min_usd: Option<f64>,
    pub mcap_min_usd: Option<f64>,
    pub vol_max_pp: Option<f64>,
    pub block_non_migrated_pumpfun: bool,
    pub dup_position_block: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevConfig {
    pub mode: String,              // "private_rpc" | "jito_bundle" | "none"
    pub cu_price_max_lamports: u64,
    pub tip_cap_lamports: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawConfig {
    pub enable: bool,
    pub threshold_usd: f64,
    pub destination: String,       // vault wallet pubkey
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalConfig {
    pub gates: GateConfig,
    pub mev: MevConfig,
    pub withdraw: WithdrawConfig,
}
