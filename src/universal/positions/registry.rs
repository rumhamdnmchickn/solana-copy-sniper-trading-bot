use std::collections::HashMap;
use std::sync::Mutex;

use super::{PositionError, PositionState, PositionStatus};

/// In-memory registry for tracking open/closed positions keyed by (wallet, mint).
///
/// This is intentionally simple and synchronous. If we need more throughput
/// later, we can switch the internal lock to an `RwLock` without changing
/// the public API.
pub struct PositionsRegistry {
    inner: Mutex<HashMap<(String, String), PositionState>>,
}

impl PositionsRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Returns `Ok(())` if a new position can be opened for the given (wallet, mint),
    /// or `Err(PositionError::AlreadyOpen)` if there is already an open position.
    pub fn can_open(&self, wallet: &str, mint: &str) -> Result<(), PositionError> {
        let guard = self.inner.lock().map_err(|e| {
            PositionError::Internal(format!("Mutex poisoned in can_open: {}", e))
        })?;

        if let Some(pos) = guard.get(&(wallet.to_string(), mint.to_string())) {
            if pos.status == PositionStatus::Open {
                return Err(PositionError::AlreadyOpen(wallet.into(), mint.into()));
            }
        }

        Ok(())
    }

    /// Records a newly opened position for the given (wallet, mint).
    ///
    /// If there is already an open position, this returns `Err(PositionError::AlreadyOpen)`.
    pub fn record_open(
        &self,
        wallet: &str,
        mint: &str,
        opened_ts: i64,
    ) -> Result<(), PositionError> {
        let mut guard = self.inner.lock().map_err(|e| {
            PositionError::Internal(format!("Mutex poisoned in record_open: {}", e))
        })?;

        if let Some(pos) = guard.get(&(wallet.to_string(), mint.to_string())) {
            if pos.status == PositionStatus::Open {
                return Err(PositionError::AlreadyOpen(wallet.into(), mint.into()));
            }
        }

        let state = PositionState {
            wallet: wallet.into(),
            mint: mint.into(),
            opened_ts,
            size: None,
            entry_price: None,
            slippage_bps: None,
            status: PositionStatus::Open,
        };

        guard.insert((wallet.into(), mint.into()), state);
        Ok(())
    }

    /// Marks an existing position as closed for the given (wallet, mint).
    ///
    /// If there is no open position, this returns `Err(PositionError::NotOpen)`.
    pub fn record_close(&self, wallet: &str, mint: &str) -> Result<(), PositionError> {
        let mut guard = self.inner.lock().map_err(|e| {
            PositionError::Internal(format!("Mutex poisoned in record_close: {}", e))
        })?;

        match guard.get_mut(&(wallet.to_string(), mint.to_string())) {
            Some(pos) => {
                if pos.status == PositionStatus::Open {
                    pos.status = PositionStatus::Closed;
                    Ok(())
                } else {
                    Err(PositionError::NotOpen(wallet.into(), mint.into()))
                }
            }
            None => Err(PositionError::NotOpen(wallet.into(), mint.into())),
        }
    /// Returns true if there is currently an open position for the given
    /// (wallet, mint) pair. If the internal mutex is poisoned, this will
    /// return `false` as a conservative default.
    pub fn has_open_position(&self, wallet: &str, mint: &str) -> bool {
        let guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        guard
            .get(&(wallet.to_string(), mint.to_string()))
            .map(|p| p.status == PositionStatus::Open)
            .unwrap_or(false)
    }

    /// Returns a clone of the current position state for the given
    /// (wallet, mint) pair, if any. This is safe to expose because the
    /// internal `PositionState` is cloned before the lock is released.
    pub fn get_open_position(&self, wallet: &str, mint: &str) -> Option<PositionState> {
        let guard = self.inner.lock().ok()?;

        guard
            .get(&(wallet.to_string(), mint.to_string()))
            .cloned()
    }

    /// Returns all open positions for the given wallet. This is intended to
    /// support read-only features such as Telegram `/positions` and risk
    /// monitoring services.
    pub fn list_open_positions_for_wallet(&self, wallet: &str) -> Vec<PositionState> {
        let guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };

        guard
            .values()
            .filter(|p| p.wallet == wallet && p.status == PositionStatus::Open)
            .cloned()
            .collect()
    }
    /// Marks all positions for the given wallet as closed.
    ///
    /// This is intended to support higher-level controls such as:
    /// - Per-wallet "exit all" commands (e.g. via Telegram)
    /// - Kill-switch style risk controls
    ///
    /// It returns the number of positions that were transitioned from
    /// `Open` to `Closed`. If the internal mutex is poisoned, this
    /// returns 0 as a conservative default.
    pub fn close_all_for_wallet(&self, wallet: &str) -> usize {
        let mut guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return 0,
        };

        let mut closed_count = 0usize;
        for state in guard.values_mut() {
            if state.wallet == wallet && state.status == PositionStatus::Open {
                state.status = PositionStatus::Closed;
                closed_count += 1;
            }
        }

        closed_count
    }


    }
}
