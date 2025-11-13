#!/usr/bin/env bash
set -euo pipefail

# ------------- helpers
file_exists(){ [[ -f "$1" ]]; }
note(){ echo "[OK] $*"; }
skip(){ echo "[SKIP] $*"; }

# ------------- A) src/block_engine/tx.rs
if file_exists src/block_engine/tx.rs; then
  # 1) Ensure zeroslot imports are feature-gated
  perl -0777 -i -pe 's~\n\s*use\s+crate::library::zeroslot::\{self,\s*ZeroSlotClient\};~\n#[cfg(feature="zeroslot")] use crate::library::zeroslot::{self, ZeroSlotClient};~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~\n\s*use\s+reqwest::\{Client,\s*ClientBuilder\};~\n#[cfg(feature="zeroslot")] use reqwest::{Client, ClientBuilder};~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~\n\s*use\s+std::time::Duration;~\n#[cfg(feature="zeroslot")] use std::time::Duration;~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~\n\s*use\s+tokio::time::\{sleep,\s*Instant\};~\n#[cfg(feature="zeroslot")] use tokio::time::{sleep, Instant};~s' src/block_engine/tx.rs || true

  # 2) Gate HTTP_CLIENT and any FLASHBLOCK_API_KEY-like constants under zeroslot
  perl -0777 -i -pe 's~(\n\s*static\s+HTTP_CLIENT:.*?Lazy::new\(\s*\{\s*).*?\}\s*\);\s*~\n#[cfg(feature="zeroslot")]$1return reqwest::Client::new();}\n);~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~\n\s*static\s+FLASHBLOCK_API_KEY:~\n#[cfg(feature="zeroslot")] static FLASHBLOCK_API_KEY:~s' src/block_engine/tx.rs || true

  # 3) Duplicate function: wrap the first dispatcher with #[cfg(feature="zeroslot")]
  # (Find the first pub async fn new_signed_and_send_with_landing_mode and gate it)
  perl -0777 -i -pe 's~(^\s*pub\s+async\s+fn\s+new_signed_and_send_with_landing_mode\s*\()~#[cfg(feature="zeroslot")]\n$&~m' src/block_engine/tx.rs

  # 4) Make sure there is a non-zeroslot fallback dispatcher (without leading blank line after attr)
  if ! grep -q 'cfg(not(feature = "zeroslot"))' src/block_engine/tx.rs; then
    cat >> src/block_engine/tx.rs <<'RS'

#[cfg(not(feature = "zeroslot"))]
pub async fn new_signed_and_send_with_landing_mode(
    _transaction_landing_mode: crate::common::config::TransactionLandingMode,
    app_state: &crate::common::config::AppState,
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &anchor_client::solana_sdk::signature::Keypair,
    instructions: Vec<anchor_client::solana_sdk::instruction::Instruction>,
    logger: &crate::common::logger::Logger,
) -> anyhow::Result<Vec<String>> {
    logger.log("Zeroslot disabled; using normal RPC landing".to_string());
    new_signed_and_send_normal(
        app_state.rpc_nonblocking_client.clone(),
        recent_blockhash,
        keypair,
        instructions,
        logger,
    ).await
}
RS
  fi

  # 5) Provide a non-feature stub for new_signed_and_send_zeroslot so callers compile w/o feature
  if ! grep -q 'cfg(not(feature = "zeroslot"))\]\s*pub\s+async\s+fn\s+new_signed_and_send_zeroslot' src/block_engine/tx.rs; then
    cat >> src/block_engine/tx.rs <<'RS'

#[cfg(not(feature = "zeroslot"))]
pub async fn new_signed_and_send_zeroslot(
    _app_state: &crate::common::config::AppState,
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &anchor_client::solana_sdk::signature::Keypair,
    instructions: Vec<anchor_client::solana_sdk::instruction::Instruction>,
    logger: &crate::common::logger::Logger,
) -> anyhow::Result<Vec<String>> {
    // Fallback: call normal path when zeroslot is disabled
    new_signed_and_send_normal(
        _app_state.rpc_nonblocking_client.clone(),
        recent_blockhash,
        keypair,
        instructions,
        logger,
    ).await
}
RS
  fi

  # 6) Remove empty line after outer attributes (clippy::empty-line-after-outer-attr)
  perl -0777 -i -pe 's/(\#\[\s*cfg[^\]]*\]\s*)\n\s*\n\s*(pub\s+async\s+fn)/$1\n$2/g' src/block_engine/tx.rs
  perl -0777 -i -pe 's/(\#\[\s*cfg_attr[^\]]*\]\s*)\n\s*\n\s*(pub\s+async\s+fn)/$1\n$2/g' src/block_engine/tx.rs

  note "patched src/block_engine/tx.rs"
fi

# ------------- B) src/processor/sniper_bot.rs
if file_exists src/processor/sniper_bot.rs; then
  # Fix wrong imports (_signature) and bring Signer trait into scope
  perl -0777 -i -pe 's/anchor_client::solana_sdk::\{pubkey::Pubkey,\s*_signature::Signature\}/anchor_client::solana_sdk::\{pubkey::Pubkey, signature::Signature\}/' src/processor/sniper_bot.rs
  perl -0777 -i -pe 's/use\s+solana_sdk::_signature::Signer;/use solana_sdk::signature::Signer;/' src/processor/sniper_bot.rs
  # In case the anchor path is used for Signer elsewhere, ensure at least one import exists:
  if ! grep -q 'use solana_sdk::signature::Signer;' src/processor/sniper_bot.rs; then
    sed -i.bak '1i\
use solana_sdk::signature::Signer;
' src/processor/sniper_bot.rs
    rm -f src/processor/sniper_bot.rs.bak
  fi

  # Replace struct field usage `_signature` -> `signature` (for geyser + TradeInfoFromToken + SellTransactionResult)
  perl -0777 -i -pe 's/\b_signature\b/signature/g' src/processor/sniper_bot.rs

  # Remove the extra blank line after a doc comment (clippy::empty-line-after-doc-comments)
  # target near line ~2316
  perl -0777 -i -pe 's/(^\/\/\/[^\n]*\n)\n/$1/sm' src/processor/sniper_bot.rs

  # Unused logger/config clones → underscore them
  perl -0777 -i -pe 's/\blet\s+config_clone\b/_config_clone/g' src/processor/sniper_bot.rs
  perl -0777 -i -pe 's/\blet\s+logger_clone\b/_logger_clone/g' src/processor/sniper_bot.rs

  note "patched src/processor/sniper_bot.rs"
fi

# ------------- C) src/processor/selling_strategy.rs & transaction_retry.rs
if file_exists src/processor/selling_strategy.rs; then
  # These files call new_signed_and_send_zeroslot. Our non-feature stub in tx.rs covers no-feature builds.
  # No change needed here unless function names drift; we keep as-is.
  note "checked selling_strategy.rs (no direct edits needed)"
fi

if file_exists src/processor/transaction_retry.rs; then
  note "checked transaction_retry.rs (no direct edits needed)"
fi

# ------------- D) src/common/config.rs, src/dex/*: unused variables/imports
if file_exists src/common/config.rs; then
  perl -0777 -i -pe 's/\blet\s+rpc_client\b/_rpc_client/g' src/common/config.rs
  # optional: if bs58 or http request import is unused, our quiet header should already allow minimal build
  note "patched src/common/config.rs"
fi

if file_exists src/dex/pump_swap.rs; then
  perl -0777 -i -pe 's/\blet\s+logger\b/_logger/g' src/dex/pump_swap.rs
  note "patched src/dex/pump_swap.rs"
fi

# ------------- E) src/processor/transaction_parser.rs
if file_exists src/processor/transaction_parser.rs; then
  # Struct init shorthand (clippy::redundant-field-names)
  perl -0777 -i -pe 's/(\bvirtual_sol_reserves):\s*\1/\1/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/(\bvirtual_token_reserves):\s*\1/\1/g' src/processor/transaction_parser.rs

  # Unused locals → underscore; remove unnecessary mut
  perl -0777 -i -pe 's/\blet\s+mut\s+is_reverse\b/let is_reverse/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\blet\s+start_time\b/let _start_time/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\bmin_quote_amount_out\b/_min_quote_amount_out/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\buser_base_token_reserves\b/_user_base_token_reserves/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\buser_quote_token_reserves\b/_user_quote_token_reserves/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\blp_fee_basis_points\b/_lp_fee_basis_points/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\blp_fee\b/_lp_fee/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\bprotocol_fee_basis_points\b/_protocol_fee_basis_points/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\bprotocol_fee\b/_protocol_fee/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\bquote_amount_out_without_lp_fee\b/_quote_amount_out_without_lp_fee/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\buser_quote_amount_out\b/_user_quote_amount_out/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\breal_token_reserves\b/_real_token_reserves/g' src/processor/transaction_parser.rs
  perl -0777 -i -pe 's/\bis_reverse_when_pump_swap\b/_is_reverse_when_pump_swap/g' src/processor/transaction_parser.rs

  note "patched src/processor/transaction_parser.rs"
fi

note "All edits applied."
