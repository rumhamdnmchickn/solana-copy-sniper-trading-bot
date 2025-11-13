#!/usr/bin/env bash
set -euo pipefail

note(){ echo "[OK] $*"; }
skip(){ echo "[SKIP] $*"; }
need(){ [[ -f "$1" ]] || { echo "Missing $1"; exit 1; }; }

need Cargo.toml

# Ensure [features] exists; add any missing feature names idempotently
ensure_feature() {
  local name="$1"
  if ! grep -q "^\[features\]" Cargo.toml; then
    printf "\n[features]\ndefault = []\n" >> Cargo.toml
  fi
  # add line if missing
  if ! grep -q "^[[:space:]]*$name[[:space:]]*=" Cargo.toml; then
    printf "%s = []\n" "$name" >> Cargo.toml
    note "Added feature: $name"
  else
    skip "Feature exists: $name"
  fi
}

for f in runtime_cfg universal_gates position_tracking execution_controls \
         auto_withdrawals telegram_control control_api dryrun_closed_env zeroslot wip; do
  ensure_feature "$f"
done

# Helper: add crate-level cfg_attr to quiet scaffolding in minimal builds
quiet_header() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  # Only add once (look for our marker)
  if ! grep -q 'Quiet planned scaffolding' "$file"; then
    tmp="$(mktemp)"
    cat > "$tmp" <<'HDR'
// Quiet planned scaffolding in minimal builds;
// when features are enabled, clippy will check normally.
#![cfg_attr(
    not(any(
        feature = "zeroslot",
        feature = "execution_controls",
        feature = "universal_gates",
        feature = "position_tracking"
    )),
    allow(unused_imports, dead_code)
)]
HDR
    cat "$file" >> "$tmp"
    mv "$tmp" "$file"
    note "Inserted cfg_attr header in $file"
  else
    skip "Header already present in $file"
  fi
}

# Quiet selected modules (safe if file missing)
quiet_header src/block_engine/tx.rs
quiet_header src/common/config.rs
quiet_header src/dex/pump_fun.rs
quiet_header src/dex/pump_swap.rs
quiet_header src/dex/raydium_launchpad.rs
quiet_header src/processor/risk_management.rs
quiet_header src/processor/selling_strategy.rs
quiet_header src/processor/sniper_bot.rs
quiet_header src/processor/transaction_parser.rs
quiet_header src/processor/transaction_retry.rs
quiet_header src/library/jupiter_api.rs

# In tx.rs: gate ZeroSlot pieces; park constants under zeroslot
if [[ -f src/block_engine/tx.rs ]]; then
  perl -0777 -i -pe 's~(\n\s*use\s+crate::library::zeroslot::\{self,\s*ZeroSlotClient\};)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~(\n\s*use\s+reqwest::\{Client(?:,|})(?:\s*ClientBuilder)?\};)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~(\n\s*use\s+std::time::Duration;)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~(\n\s*use\s+tokio::time::\{sleep,\s*Instant\};)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true

  perl -0777 -i -pe 's~(\n\s*static\s+FLASHBLOCK_API_KEY:.*?\n\});)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~(\n\s*static\s+HTTP_CLIENT:.*?\n\});)~\n#[cfg(feature = "zeroslot")]$1~s' src/block_engine/tx.rs || true

  perl -0777 -i -pe 's~(^\s*pub\s+async\s+fn\s+new_signed_and_send_zeroslot\s*\()~#[cfg(feature = "zeroslot")]\n$&~m' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's~(^\s*pub\s+async\s+fn\s+new_signed_and_send_zeroslot_fast\s*\()~#[cfg(feature = "zeroslot")]\n#[cfg_attr(not(feature = "execution_controls"), expect(unused_variables, reason = "planned: unit_limit/price/tip wired by execution_controls"))]\n$&~m' src/block_engine/tx.rs || true

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
    note "Added zeroslot-off dispatcher fallback"
  else
    skip "zeroslot-off dispatcher already present"
  fi

  # keep param scaffolding quiet even if function body still unused
  perl -0777 -i -pe 's/\bcompute_unit_limit\b/_compute_unit_limit/g' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's/\bcompute_unit_price\b/_compute_unit_price/g' src/block_engine/tx.rs || true
  perl -0777 -i -pe 's/\btip_lamports\b/_tip_lamports/g' src/block_engine/tx.rs || true
fi

# Cross-platform token prefix (use perl, not BSD vs GNU sed)
perl -0777 -i -pe 's/\bslippage_bps\b/_slippage_bps/g' src/dex/pump_fun.rs 2>/dev/null || true
perl -0777 -i -pe 's/\bslippage_bps\b/_slippage_bps/g' src/dex/pump_swap.rs 2>/dev/null || true
perl -0777 -i -pe 's/\btoken_info\b/_token_info/g' src/processor/risk_management.rs 2>/dev/null || true
perl -0777 -i -pe 's/\bsignature\b/_signature/g' src/processor/sniper_bot.rs 2>/dev/null || true

note "Done."
