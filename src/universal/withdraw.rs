
use std::time::Duration;
use tokio::time::interval;

/// Periodic withdrawals placeholder
pub async fn run_withdrawals_task(_vault: String, _min_daily_usd: f64) {
    let mut ticker = interval(Duration::from_secs(60 * 60)); // hourly for now
    loop {
        ticker.tick().await;
        // TODO: compute unsettled PnL & balances, if > threshold, craft tx to vault
        // Use Jupiter swap if tokens != SOL/USDC
        // Persist outcomes to logs
    }
}
