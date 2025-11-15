//! Basic integration tests for the in-memory positions registry.
//!
//! These tests exercise the core invariant we care about in Phase 1A:
//! - No duplicate open positions per (wallet, mint) until the position is closed.

use solana_vntr_sniper::universal::positions::{PositionError, PositionsRegistry};

#[test]
fn fresh_registry_allows_opening_position() {
    let registry = PositionsRegistry::new();

    // Initially, we should be allowed to open a position for (walletA, mintX).
    assert!(registry.can_open("walletA", "mintX").is_ok());
    assert!(registry.record_open("walletA", "mintX", 1_700_000_000).is_ok());
}

#[test]
fn duplicate_open_is_blocked_until_closed() {
    let registry = PositionsRegistry::new();

    registry
        .record_open("walletA", "mintX", 1_700_000_000)
        .expect("initial open should succeed");

    // Attempting to check can_open again should fail with AlreadyOpen.
    let can_open_again = registry.can_open("walletA", "mintX");
    assert!(matches!(
        can_open_again,
        Err(PositionError::AlreadyOpen(wallet, mint))
        if wallet == "walletA" && mint == "mintX"
    ));

    // Attempting to record_open again should also fail with AlreadyOpen.
    let open_again = registry.record_open("walletA", "mintX", 1_700_000_001);
    assert!(matches!(
        open_again,
        Err(PositionError::AlreadyOpen(wallet, mint))
        if wallet == "walletA" && mint == "mintX"
    ));
}

#[test]
fn closing_position_allows_reopen() {
    let registry = PositionsRegistry::new();

    registry
        .record_open("walletA", "mintX", 1_700_000_000)
        .expect("initial open should succeed");

    registry
        .record_close("walletA", "mintX")
        .expect("closing existing position should succeed");

    // After closing, we should be able to open again for the same (wallet, mint).
    assert!(registry.can_open("walletA", "mintX").is_ok());
    assert!(registry.record_open("walletA", "mintX", 1_700_000_010).is_ok());
}

#[test]
fn closing_non_open_position_returns_not_open() {
    let registry = PositionsRegistry::new();

    // No position was opened yet, so closing should return NotOpen.
    let res = registry.record_close("walletA", "mintX");
    assert!(matches!(
        res,
        Err(PositionError::NotOpen(wallet, mint))
        if wallet == "walletA" && mint == "mintX"
    ));
}
