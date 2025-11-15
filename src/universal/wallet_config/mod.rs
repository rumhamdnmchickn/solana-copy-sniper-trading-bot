//! Wallet-level configuration (slippage, TP/SL, max positions, etc.).
//!
//! Phase 1: type definitions and an empty in-memory map.
//! Later phases will wire this into src/common/config.rs and Telegram controls.

pub mod types;

pub use types::{WalletConfig, WalletConfigMap, EffectiveWalletParams};
use once_cell::sync::Lazy;

/// Global wallet-configuration map.
///
/// Phase 1: populated via `from_env_or_empty()`, which currently returns an empty map.
/// Later phases will load from env/config (e.g. WALLET_CONFIG_TOML) and possibly
/// expose additional helper accessors.
pub static GLOBAL_WALLET_CONFIGS: Lazy<WalletConfigMap> =
    Lazy::new(|| WalletConfigMap::from_env_or_empty());

/// Accessor for the global wallet-configuration map.
#[inline]
pub fn get_wallet_config_map() -> &'static WalletConfigMap {
    &GLOBAL_WALLET_CONFIGS
}
/// Return the configured `max_positions` for this wallet, if any.
///
/// This is just a convenience wrapper around the global wallet-config map.
#[inline]
pub fn effective_max_positions_for_wallet(wallet: &str) -> Option<u32> {
    get_wallet_config_map().get_max_positions(wallet)
}

/// Count how many open positions this wallet currently has.
///
/// Phase 1: this is a stub that always returns 0. Later phases can wire this
/// into the real positions registry once the module path is stable.
pub fn open_position_count_for_wallet(_wallet: &str) -> usize {
    0
}

/// Whether this wallet is allowed to open at least one more position under the
/// current `max_positions` configuration.
///
/// Semantics:
/// - If `max_positions` is `None`, there is effectively no limit → returns true.
/// - If `max_positions` is Some(n), we allow opening a new position only when
///   the current open count is strictly less than `n`.
pub fn can_open_more_positions_for_wallet(wallet: &str) -> bool {
    let current = open_position_count_for_wallet(wallet);
    match effective_max_positions_for_wallet(wallet) {
        Some(max) if max > 0 => current < max as usize,
        Some(_) => {
            // Treat zero or otherwise odd values as "no additional positions".
            // This is conservative and can be revisited later if needed.
            false
        }
        None => true,
    }
}
/// Resolve effective parameters (slippage, TP/SL, max_positions) for a wallet,
/// using wallet-specific overrides layered on top of the provided defaults.
///
/// This is a convenience wrapper around the global wallet-config map and
/// `WalletConfigMap::resolve_params_for_wallet`.
#[inline]
pub fn resolve_effective_params_for_wallet(
    wallet: &str,
    default_slippage: f64,
    default_tp: f64,
    default_sl: f64,
    default_max_positions: Option<u32>,
) -> EffectiveWalletParams {
    get_wallet_config_map().resolve_params_for_wallet(
        wallet,
        default_slippage,
        default_tp,
        default_sl,
        default_max_positions,
    )
}
