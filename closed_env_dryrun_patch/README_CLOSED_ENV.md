# Closed Environment / Dry-Run & Replay Patch

This bundle adds:
- `EXECUTION_MODE` (DRY_RUN | SIMULATE | LIVE)
- WebSocket recording (`RECORD_WS`, `WS_RECORD_PATH`)
- Offline replay binary (`cargo run --bin replay_ws`) to drive the bot metrics without any network or trading

## How to Use

1) Append the variables from `ENV_EXAMPLE_APPEND.txt` to your `.env` (leave EXECUTION_MODE=DRY_RUN).
2) Add the modules:
   - Copy `src/execution/mod.rs` into your repo.
   - Copy `src/ingest/ws_tap.rs` and import it in `src/ingest/birdeye_ws.rs`; call `ws_tap::record_line(&txt)` inside your WS reader loop before parsing.
   - Copy `src/bin/replay_ws.rs` and replace `your_crate` with your actual crate name in the `use` lines.
3) In your `sniper_bot.rs` send path, apply `PATCH_sniper_executor.diff` (replace the direct RPC send with `exec.execute(...)`).
4) For SIMULATE mode, wire a real `RpcClient` and call `simulate_transaction` with the Jupiter-built transaction.

## Test Modes

- **Dry-Run (recommended first):** live WS, decisions logged, no network sends.
  - Set `EXECUTION_MODE=DRY_RUN`, run normally. Verify that guard passes/fails and sizing behave correctly.
- **Simulate:** call RPC `simulateTransaction` for each built swap. You can assert logs and post-token balance deltas without sending.
  - Set `EXECUTION_MODE=SIMULATE` and implement the TODO in `SimExecutor`.
- **Offline Replay (air-gapped):**
  - Run once in DRY_RUN with `RECORD_WS=1` to capture frames.
  - Then disconnect the network and run `cargo run --bin replay_ws` to repopulate live metrics from the JSONL file.
  - Your guard + sizing code will behave as if the stream were live.

## Safety
- DRY_RUN guarantees no `send_transaction` is called.
- SIMULATE must not fall back to `send` on failure. Keep `skip_preflight=false` in live mode.