use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PositionStatus {
    Open,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionState {
    /// Wallet public key in base58 string form.
    pub wallet: String,
    /// Token mint public key in base58 string form.
    pub mint: String,
    /// Unix timestamp (seconds) when the position was opened.
    pub opened_ts: i64,
    /// Optional position size (tokens or quote units). Reserved for later use.
    pub size: Option<f64>,
    /// Optional entry price (in quote units). Reserved for later use.
    pub entry_price: Option<f64>,
    /// Optional slippage in basis points. Reserved for later use.
    pub slippage_bps: Option<f64>,
    /// Current status of the position.
    pub status: PositionStatus,
}
