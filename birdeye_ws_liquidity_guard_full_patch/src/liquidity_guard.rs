use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub mint: String,

    // Rolling USD volumes
    pub vol_5m_usd: f64,
    pub vol_15m_usd: f64,

    // Liquidity / depth proxy, e.g. best bid/ask depth in USD
    pub liq_usd: Option<f64>,

    // Market cap / FDV proxy in USD
    pub marketcap_usd: Option<f64>,

    // Amihud-style illiquidity score (abs(dP)/vol_usd)
    pub amihud_5m: Option<f64>,

    // Range efficiency 0..1 (directionality vs noise)
    pub range_eff_5m: Option<f64>,

    // When were these metrics last refreshed?
    pub last_update: DateTime<Utc>,
}

// Thresholds we read from env / config
#[derive(Debug, Clone)]
pub struct GuardThresholds {
    pub liq_5m_min_usd: f64,
    pub liq_15m_min_usd: f64,
    pub min_mcap_usd: f64,
    pub amihud_max: f64,
    pub range_eff_min: f64,

    // how stale before we refuse?
    pub max_age_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDecision {
    pub allowed: bool,
    pub reason: String,

    pub mint: String,
    pub requested_size_usd: f64,

    pub vol_5m_usd: f64,
    pub vol_15m_usd: f64,
    pub liq_usd: Option<f64>,
    pub marketcap_usd: Option<f64>,
    pub amihud_5m: Option<f64>,
    pub range_eff_5m: Option<f64>,
}

// Simple helper to compute "can we buy?"
pub fn evaluate_guard(
    mint: &str,
    requested_size_usd: f64,
    tm: &TokenMetrics,
    th: &GuardThresholds,
    now: DateTime<Utc>,
) -> GuardDecision {
    // staleness check first
    let age = (now - tm.last_update).num_seconds();
    if age > th.max_age_secs {
        return GuardDecision {
            allowed: false,
            reason: format!("stale_metrics age_secs={}", age),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }

    // rolling volume guards (liquidity-in-time)
    if tm.vol_5m_usd < th.liq_5m_min_usd {
        return GuardDecision {
            allowed: false,
            reason: format!("low_liq_5m {} < {}", tm.vol_5m_usd, th.liq_5m_min_usd),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }
    if tm.vol_15m_usd < th.liq_15m_min_usd {
        return GuardDecision {
            allowed: false,
            reason: format!("low_liq_15m {} < {}", tm.vol_15m_usd, th.liq_15m_min_usd),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }

    // hard market cap floor
    if let Some(mc) = tm.marketcap_usd {
        if mc < th.min_mcap_usd {
            return GuardDecision {
                allowed: false,
                reason: format!("low_mcap {} < {}", mc, th.min_mcap_usd),
                mint: mint.to_string(),
                requested_size_usd,
                vol_5m_usd: tm.vol_5m_usd,
                vol_15m_usd: tm.vol_15m_usd,
                liq_usd: tm.liq_usd,
                marketcap_usd: tm.marketcap_usd,
                amihud_5m: tm.amihud_5m,
                range_eff_5m: tm.range_eff_5m,
            };
        }
    } else {
        return GuardDecision {
            allowed: false,
            reason: "no_mcap".to_string(),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }

    // Amihud (slippage proxy). We block if it's too illiquid (too high).
    if let Some(amihud) = tm.amihud_5m {
        if amihud > th.amihud_max {
            return GuardDecision {
                allowed: false,
                reason: format!("amihud {} > {}", amihud, th.amihud_max),
                mint: mint.to_string(),
                requested_size_usd,
                vol_5m_usd: tm.vol_5m_usd,
                vol_15m_usd: tm.vol_15m_usd,
                liq_usd: tm.liq_usd,
                marketcap_usd: tm.marketcap_usd,
                amihud_5m: tm.amihud_5m,
                range_eff_5m: tm.range_eff_5m,
            };
        }
    } else {
        return GuardDecision {
            allowed: false,
            reason: "no_amihud".to_string(),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }

    // Range efficiency (volatility sanity). Low => chaotic candles / rug spike.
    if let Some(reff) = tm.range_eff_5m {
        if reff < th.range_eff_min {
            return GuardDecision {
                allowed: false,
                reason: format!("range_eff {} < {}", reff, th.range_eff_min),
                mint: mint.to_string(),
                requested_size_usd,
                vol_5m_usd: tm.vol_5m_usd,
                vol_15m_usd: tm.vol_15m_usd,
                liq_usd: tm.liq_usd,
                marketcap_usd: tm.marketcap_usd,
                amihud_5m: tm.amihud_5m,
                range_eff_5m: tm.range_eff_5m,
            };
        }
    } else {
        return GuardDecision {
            allowed: false,
            reason: "no_range_eff".to_string(),
            mint: mint.to_string(),
            requested_size_usd,
            vol_5m_usd: tm.vol_5m_usd,
            vol_15m_usd: tm.vol_15m_usd,
            liq_usd: tm.liq_usd,
            marketcap_usd: tm.marketcap_usd,
            amihud_5m: tm.amihud_5m,
            range_eff_5m: tm.range_eff_5m,
        };
    }

    GuardDecision {
        allowed: true,
        reason: "ok".to_string(),
        mint: mint.to_string(),
        requested_size_usd,
        vol_5m_usd: tm.vol_5m_usd,
        vol_15m_usd: tm.vol_15m_usd,
        liq_usd: tm.liq_usd,
        marketcap_usd: tm.marketcap_usd,
        amihud_5m: tm.amihud_5m,
        range_eff_5m: tm.range_eff_5m,
    }
}

/// Helper to load thresholds from env or fallback defaults
pub fn load_thresholds_from_env() -> GuardThresholds {
    let liq_5m_min_usd = std::env::var("LIQ_5M_MIN_USD")
        .unwrap_or_else(|_| "15000".into())
        .parse().unwrap_or(15000.0);

    let liq_15m_min_usd = std::env::var("LIQ_15M_MIN_USD")
        .unwrap_or_else(|_| "45000".into())
        .parse().unwrap_or(45000.0);

    let min_mcap_usd = std::env::var("MIN_MCAP_USD")
        .unwrap_or_else(|_| "5000000".into())
        .parse().unwrap_or(5_000_000.0);

    let amihud_max = std::env::var("AMIHUD_MAX")
        .unwrap_or_else(|_| "0.8".into())
        .parse().unwrap_or(0.8);

    let range_eff_min = std::env::var("RANGE_EFF_MIN")
        .unwrap_or_else(|_| "0.35".into())
        .parse().unwrap_or(0.35);

    let max_age_secs = 60_i64 * 5; // refuse if older than 5 minutes

    GuardThresholds {
        liq_5m_min_usd,
        liq_15m_min_usd,
        min_mcap_usd,
        amihud_max,
        range_eff_min,
        max_age_secs,
    }
}
