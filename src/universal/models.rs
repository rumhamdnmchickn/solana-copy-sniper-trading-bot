
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimInput {
    pub ts: i64,
    pub target_wallet: String,
    pub mint: String,
    pub side: String,       // buy/sell
    pub qty: f64,
    pub price_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimResult {
    pub ts: i64,
    pub mint: String,
    pub filled: bool,
    pub reason: Option<String>,
    pub exit_ts: Option<i64>,
    pub pnl_usd: f64,
}
