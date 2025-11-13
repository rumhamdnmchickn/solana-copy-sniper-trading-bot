#!/usr/bin/env python3
"""
Patch src/processor/sniper_bot.rs to add retry logic for failed buy transactions.

What this does:

1. Adds a new wrapper function:

       const EXECUTE_BUY_MAX_RETRIES: u32 = 3;

       pub async fn execute_buy_with_retry(
           trade_info: transaction_parser::TradeInfoFromToken,
           app_state: Arc<AppState>,
           swap_config: Arc<SwapConfig>,
           protocol: SwapProtocol,
       ) -> Result<(), String>

   This wrapper:
   - Calls the existing execute_buy(...)
   - Retries up to EXECUTE_BUY_MAX_RETRIES times on error
   - Logs each attempt and final failure

2. Replaces call sites of `match execute_buy(` with `match execute_buy_with_retry(`.

   This affects the main processing paths where buys are initiated, but
   leaves the core execute_buy logic and its internal behavior intact.

The patch is idempotent:
- If `execute_buy_with_retry` already exists, it won't be re-added.
- If call sites already use `execute_buy_with_retry`, they won't be changed again.
"""

from pathlib import Path

SNIPER_PATH = Path("src/processor/sniper_bot.rs")


WRAPPER_SNIPPET = r"""
// Maximum number of attempts for executing a buy transaction.
// This is a simple global control; can be later made configurable per wallet.
const EXECUTE_BUY_MAX_RETRIES: u32 = 3;

/// Execute buy with simple retry logic.
/// Retries the existing `execute_buy` up to EXECUTE_BUY_MAX_RETRIES times
/// if it returns an error (e.g., exceeded slippage allowance or transient RPC issues).
pub async fn execute_buy_with_retry(
    trade_info: transaction_parser::TradeInfoFromToken,
    app_state: Arc<AppState>,
    swap_config: Arc<SwapConfig>,
    protocol: SwapProtocol,
) -> Result<(), String> {
    let logger = Logger::new("[EXECUTE-BUY-RETRY] => ".green().to_string());
    let mut attempt: u32 = 0;

    loop {
        attempt += 1;

        logger.log(
            format!(
                "🔄 Buy attempt {} for token {}",
                attempt,
                trade_info.mint
            )
            .cyan()
            .to_string(),
        );

        match execute_buy(
            trade_info.clone(),
            app_state.clone(),
            swap_config.clone(),
            protocol.clone(),
        ).await {
            Ok(_) => {
                if attempt > 1 {
                    logger.log(
                        format!(
                            "✅ Buy succeeded on attempt {} for token {}",
                            attempt,
                            trade_info.mint
                        )
                        .green()
                        .to_string(),
                    );
                }
                return Ok(());
            }
            Err(e) => {
                if attempt >= EXECUTE_BUY_MAX_RETRIES {
                    logger.log(
                        format!(
                            "❌ Buy failed after {} attempts for token {}: {}",
                            attempt,
                            trade_info.mint,
                            e
                        )
                        .red()
                        .to_string(),
                    );
                    return Err(format!(
                        "Buy failed after {} attempts: {}",
                        attempt, e
                    ));
                } else {
                    logger.log(
                        format!(
                            "⚠️ Buy attempt {} failed for token {}: {}. Retrying...",
                            attempt,
                            trade_info.mint,
                            e
                        )
                        .yellow()
                        .to_string(),
                    );
                    // Small delay before retrying; tuned conservatively.
                    time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }
}

"""


def add_wrapper(content: str) -> str:
    """Insert execute_buy_with_retry wrapper before the execute_buy doc comment."""
    if "pub async fn execute_buy_with_retry" in content:
        # Already patched
        return content

    marker = "/// Execute buy operation based on detected transaction"
    idx = content.find(marker)
    if idx == -1:
        raise SystemExit(
            "Could not find execute_buy doc comment marker; aborting retry patch."
        )

    # Insert wrapper snippet just before the marker
    return content[:idx] + WRAPPER_SNIPPET + content[idx:]


def patch_call_sites(content: str) -> str:
    """
    Replace call sites that do `match execute_buy(` with `match execute_buy_with_retry(`.

    We restrict to the exact pattern "match execute_buy(" to avoid touching the
    function definition.
    """
    if "match execute_buy_with_retry(" in content:
        # Assume call sites already patched
        return content

    old = "match execute_buy(\n"
    new = "match execute_buy_with_retry(\n"
    if old not in content:
        # Nothing to patch; maybe structure changed
        print("Warning: did not find any `match execute_buy(` call sites to patch.")
        return content

    return content.replace(old, new)


def main() -> None:
    if not SNIPER_PATH.exists():
        raise SystemExit(
            f"File not found: {SNIPER_PATH}. Run this script from the repo root."
        )

    original = SNIPER_PATH.read_text()

    patched = add_wrapper(original)
    patched = patch_call_sites(patched)

    if patched == original:
        print("No changes made: sniper_bot.rs already appears to have retry logic wired.")
        return

    backup_path = SNIPER_PATH.with_suffix(SNIPER_PATH.suffix + ".bak_retry")
    backup_path.write_text(original)
    SNIPER_PATH.write_text(patched)

    print(f"Patched {SNIPER_PATH}")
    print(f"Backup written to {backup_path}")


if __name__ == "__main__":
    main()
