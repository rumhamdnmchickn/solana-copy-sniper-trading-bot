use std::collections::HashMap;
use std::env;

use serde::Deserialize;

/// Per-wallet configuration (slippage, TP/SL, max open positions).
///
/// Loaded from a TOML blob in the WALLET_CONFIG_TOML environment variable,
/// shaped like:
///
/// [wallets.<pubkey>]
/// slippage = 0.02
/// tp       = 1.25
/// sl       = 0.20
/// max_positions = 2
#[derive(Debug, Clone, Deserialize)]
pub struct WalletConfig {
    /// Per-wallet slippage as a fraction, e.g. 0.02 = 2%.
    pub slippage: Option<f64>,
    /// Take-profit multiple relative to entry, e.g. 1.25 = +25% profit.
    pub tp: Option<f64>,
    /// Stop-loss multiple relative to entry, e.g. 0.20 = keep 20% of entry.
    pub sl: Option<f64>,
    /// Maximum simultaneously open positions for this wallet.
    pub max_positions: Option<u32>,
}

impl WalletConfig {
    /// An "empty" config: no overrides for any field.
    #[inline]
    pub fn empty() -> Self {
        Self {
            slippage: None,
            tp: None,
            sl: None,
            max_positions: None,
        }
    }
}

/// Helper struct that mirrors the TOML layout:
///
/// [wallets.<pubkey>]
/// ...
#[derive(Debug, Default, Deserialize)]
struct WalletConfigFile {
    #[serde(default)]
    wallets: HashMap<String, WalletConfig>,
}

/// Map from wallet pubkey (string) to its configuration.
#[derive(Debug, Default)]
pub struct WalletConfigMap {
    inner: HashMap<String, WalletConfig>,
}

impl WalletConfigMap {
    /// Construct an empty map (no per-wallet overrides).
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Convenience alias when we only care that it's empty.
    pub fn empty() -> Self {
        Self::new()
    }

    /// Raw inner map (useful for iteration, debugging).
    pub fn inner(&self) -> &HashMap<String, WalletConfig> {
        &self.inner
    }

    /// Get config for a wallet, if any.
    pub fn get(&self, wallet: &str) -> Option<&WalletConfig> {
        self.inner.get(wallet)
    }

    /// Get config for a wallet, or an all-None "empty" config if missing.
    pub fn get_or_default(&self, wallet: &str) -> WalletConfig {
        self.get(wallet)
            .cloned()
            .unwrap_or_else(WalletConfig::empty)
    }

    /// Insert or replace per-wallet configuration.
    pub fn insert(&mut self, wallet: String, cfg: WalletConfig) {
        self.inner.insert(wallet, cfg);
    }

    /// Convenience: get just the slippage override for a wallet, if any.
    #[inline]
    pub fn get_slippage(&self, wallet: &str) -> Option<f64> {
        self.get(wallet).and_then(|cfg| cfg.slippage)
    }

    /// Convenience: get just the take-profit multiple override for a wallet, if any.
    #[inline]
    pub fn get_tp(&self, wallet: &str) -> Option<f64> {
        self.get(wallet).and_then(|cfg| cfg.tp)
    }

    /// Convenience: get just the stop-loss multiple override for a wallet, if any.
    #[inline]
    pub fn get_sl(&self, wallet: &str) -> Option<f64> {
        self.get(wallet).and_then(|cfg| cfg.sl)
    }

    /// Convenience: get just the max_positions override for a wallet, if any.
    #[inline]
    pub fn get_max_positions(&self, wallet: &str) -> Option<u32> {
        self.get(wallet).and_then(|cfg| cfg.max_positions)
    }

    /// Load from WALLET_CONFIG_TOML env var, or return an empty map on failure.
    ///
    /// Expected TOML shape:
    ///
    /// [wallets.<pubkey>]
    /// slippage = 0.02
    /// tp       = 1.25
    /// sl       = 0.20
    /// max_positions = 2
    pub fn from_env_or_empty() -> Self {
        match env::var("WALLET_CONFIG_TOML") {
            Ok(raw) if !raw.trim().is_empty() => {
                match toml::from_str::<WalletConfigFile>(&raw) {
                    Ok(file) => {
                        let mut map = WalletConfigMap::new();
                        for (wallet, cfg) in file.wallets {
                            map.insert(wallet, cfg);
                        }
                        map
                    }
                    Err(err) => {
                        // Avoid panicking on bad config; just log and fallback.
                        eprintln!("Failed to parse WALLET_CONFIG_TOML: {err}");
                        WalletConfigMap::empty()
                    }
                }
            }
            _ => WalletConfigMap::empty(),
        }
    }
}
/// Fully-resolved per-wallet parameters combining defaults and overrides.
///
/// Typical usage:
/// - Callers pass their "global" defaults (e.g. from src/common/config.rs).
/// - Wallet-specific overrides from WALLET_CONFIG_TOML take precedence.
#[derive(Debug, Clone)]
pub struct EffectiveWalletParams {
    /// Final slippage *fraction* to use for this wallet, e.g. 0.02 = 2%.
    pub slippage: f64,
    /// Final take-profit multiple (e.g. 1.25 = +25% profit).
    pub tp: f64,
    /// Final stop-loss multiple (e.g. 0.20 = keep 20% of entry).
    pub sl: f64,
    /// Final max open-positions limit for this wallet.
    /// None means "no explicit limit" at the config layer.
    pub max_positions: Option<u32>,
}

impl WalletConfigMap {
    /// Resolve final per-wallet parameters using configured overrides layered
    /// on top of the provided defaults.
    ///
    /// For each field:
    /// - If the wallet has an override, it wins.
    /// - Otherwise, the default_* argument is used.
    pub fn resolve_params_for_wallet(
        &self,
        wallet: &str,
        default_slippage: f64,
        default_tp: f64,
        default_sl: f64,
        default_max_positions: Option<u32>,
    ) -> EffectiveWalletParams {
        let cfg = self.get(wallet);

        EffectiveWalletParams {
            slippage: cfg
                .and_then(|c| c.slippage)
                .unwrap_or(default_slippage),
            tp: cfg
                .and_then(|c| c.tp)
                .unwrap_or(default_tp),
            sl: cfg
                .and_then(|c| c.sl)
                .unwrap_or(default_sl),
            max_positions: cfg
                .and_then(|c| c.max_positions)
                .or(default_max_positions),
        }
    }
}
