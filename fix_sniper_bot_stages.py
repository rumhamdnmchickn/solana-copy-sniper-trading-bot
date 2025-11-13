#!/usr/bin/env python3
import argparse
from pathlib import Path

SNIPER_PATH = Path("src/processor/sniper_bot.rs")


def backup(path: Path):
    bak = path.with_suffix(path.suffix + ".bak")
    if not bak.exists():
        bak.write_text(path.read_text(encoding="utf-8"), encoding="utf-8")
        print(f"[backup] wrote {bak}")
    else:
        print(f"[backup] {bak} already exists; not overwriting")


# ---------- STAGE 1 ----------
# - inner -> outer cfg_attr
# - remove duplicate imports
# - remove stub `pub struct BoughtTokenInfo;`
def stage1():
    if not SNIPER_PATH.exists():
        raise SystemExit(f"File not found: {SNIPER_PATH}")

    backup(SNIPER_PATH)
    text = SNIPER_PATH.read_text(encoding="utf-8")

    # 1) inner -> outer attribute: #![cfg_attr(..)] -> #[cfg_attr(..)]
    if "#![cfg_attr(" in text:
        print("[stage1] fixing inner cfg_attr attribute")
        text = text.replace("#![cfg_attr(", "#[cfg_attr(", 1)
    else:
        print("[stage1] cfg_attr attribute already looks ok (no #![cfg_attr found)")

    # 2) remove *later* duplicate imports for specific lines
    dup_lines = [
        "use std::sync::atomic::{AtomicBool, Ordering};\n",
        "use std::sync::Arc;\n",
        "use dashmap::DashMap;\n",
        "use tokio_util::sync::CancellationToken;\n",
    ]

    for line in dup_lines:
        first = text.find(line)
        if first == -1:
            continue
        second = text.find(line, first + len(line))
        if second != -1:
            print(f"[stage1] removing duplicate import: {line.strip()}")
        while second != -1:
            text = text[:second] + text[second + len(line):]
            second = text.find(line, first + len(line))

    # 3) shrink config import: remove AppState / SwapConfig duplicates in use block
    old_cfg = "    config::{AppState, Config, SwapConfig},\n"
    new_cfg = "    config::{Config},\n"
    if old_cfg in text:
        print("[stage1] simplifying config import (removing AppState, SwapConfig duplicates)")
        text = text.replace(old_cfg, new_cfg)

    # 4) remove stub `pub struct BoughtTokenInfo;`
    if "pub struct BoughtTokenInfo;\n" in text:
        print("[stage1] removing stub `pub struct BoughtTokenInfo;`")
        text = text.replace("pub struct BoughtTokenInfo;\n", "")

    SNIPER_PATH.write_text(text, encoding="utf-8")
    print("[stage1] done.")


# ---------- STAGE 2 ----------
# - add lazy_static imports & globals for COUNTER, SOLD_TOKENS, etc.
# - add stub BOUGHT_TOKEN_LIST
def stage2():
    if not SNIPER_PATH.exists():
        raise SystemExit(f"File not found: {SNIPER_PATH}")

    backup(SNIPER_PATH)
    text = SNIPER_PATH.read_text(encoding="utf-8")

    lines = text.splitlines(keepends=True)

    # 1) ensure required use lines exist
    def ensure_use(line_str: str):
        nonlocal lines
        if any(l.strip() == line_str for l in lines):
            return
        # insert after last `use ` line
        last_use_idx = max(
            (i for i, l in enumerate(lines) if l.lstrip().startswith("use ")),
            default=-1,
        )
        insert_line = line_str + "\n"
        insert_at = last_use_idx + 1
        print(f"[stage2] adding import: {line_str}")
        lines.insert(insert_at, insert_line)

    ensure_use("use lazy_static::lazy_static;")
    ensure_use("use dashmap::DashMap;")
    ensure_use("use std::sync::{Arc, atomic::{AtomicBool, Ordering}};")
    ensure_use("use tokio_util::sync::CancellationToken;")
    ensure_use("use std::time::Instant;")

    text = "".join(lines)

    # 2) add lazy_static block if not present
    if "lazy_static! {" not in text:
        print("[stage2] inserting lazy_static globals")
        lazy_block = """

lazy_static! {
    static ref COUNTER: DashMap<(), u64> = DashMap::new();
    static ref SOLD_TOKENS: DashMap<(), u64> = DashMap::new();
    static ref BOUGHT_TOKENS: DashMap<(), u64> = DashMap::new();
    static ref LAST_BUY_TIME: DashMap<(), Option<Instant>> = DashMap::new();
    static ref BUYING_ENABLED: DashMap<(), bool> = DashMap::new();

    // maps token_mint -> (token_name, cancellation_token)
    static ref MONITORING_TASKS: DashMap<String, (String, CancellationToken)> = DashMap::new();

    static ref SHOULD_CONTINUE_STREAMING: AtomicBool = AtomicBool::new(true);
}
"""
        lines = text.splitlines(keepends=True)
        last_use_idx = max(
            (i for i, l in enumerate(lines) if l.lstrip().startswith("use ")),
            default=-1,
        )
        insert_at = last_use_idx + 1
        lines.insert(insert_at, lazy_block + "\n")
        text = "".join(lines)
    else:
        print("[stage2] lazy_static! block already present; skipping insertion")

    # 3) stub BOUGHT_TOKEN_LIST if missing
    if "pub static BOUGHT_TOKEN_LIST" not in text:
        print("[stage2] adding stub `pub static BOUGHT_TOKEN_LIST: () = ();`")
        text += "\n\npub static BOUGHT_TOKEN_LIST: () = ();\n"
    else:
        print("[stage2] BOUGHT_TOKEN_LIST static already present; skipping")

    SNIPER_PATH.write_text(text, encoding="utf-8")
    print("[stage2] done.")


# ---------- helper for Stage 3 ----------
def comment_out_function_duplicates(signature: str):
    """Keep first occurrence of a function and comment-out later duplicates.

    signature: e.g. 'pub async fn check_and_stop_streaming_if_all_sold'
    """
    text = SNIPER_PATH.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    # find line indices containing the signature
    indices = [i for i, l in enumerate(lines) if signature in l]
    if len(indices) <= 1:
        print(f"[stage3] no duplicates to remove for: {signature}")
        return

    print(f"[stage3] found {len(indices)} occurrences of {signature}; keeping first, commenting others")
    # keep first, comment-out subsequent
    for start_idx in indices[1:]:
        # find end of function body by brace depth
        depth = 0
        started = False
        end_idx = start_idx
        for j in range(start_idx, len(lines)):
            line = lines[j]
            for ch in line:
                if ch == '{':
                    depth += 1
                    started = True
                elif ch == '}':
                    depth -= 1
            end_idx = j
            if started and depth == 0:
                break
        # comment-out from start_idx to end_idx
        for k in range(start_idx, end_idx + 1):
            if not lines[k].lstrip().startswith("//"):
                lines[k] = "// " + lines[k]

    SNIPER_PATH.write_text("".join(lines), encoding="utf-8")


# ---------- STAGE 3 ----------
# - remove duplicate helper functions at bottom
def stage3():
    if not SNIPER_PATH.exists():
        raise SystemExit(f"File not found: {SNIPER_PATH}")
    backup(SNIPER_PATH)

    comment_out_function_duplicates("pub async fn check_and_stop_streaming_if_all_sold")
    comment_out_function_duplicates("pub async fn execute_enhanced_sell")

    print("[stage3] done.")


# ---------- STAGE 4 ----------
# - small syntax cleanups (e.g. `Ok(_) => Ok(()),,` -> `Ok(_) => Ok(()),`)
def stage4():
    if not SNIPER_PATH.exists():
        raise SystemExit(f"File not found: {SNIPER_PATH}")
    backup(SNIPER_PATH)

    text = SNIPER_PATH.read_text(encoding="utf-8")

    # fix double commas
    if "Ok(_) => Ok(()),," in text:
        print("[stage4] fixing `Ok(_) => Ok(()),,` -> `Ok(_) => Ok(()),`")
        text = text.replace("Ok(_) => Ok(()),,", "Ok(_) => Ok(()),")

    # (we intentionally keep Stage 4 conservative and do only obviously-safe string fixes)

    SNIPER_PATH.write_text(text, encoding="utf-8")
    print("[stage4] done.")


def main():
    parser = argparse.ArgumentParser(description="Stage-wise fixer for src/processor/sniper_bot.rs")
    parser.add_argument(
        "--stage",
        type=int,
        choices=[1, 2, 3, 4],
        help="Run a single stage (1-4)",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Run all stages in order",
    )
    args = parser.parse_args()

    if args.all:
        print("=== Running ALL stages (1 -> 4) ===")
        stage1()
        stage2()
        stage3()
        stage4()
    elif args.stage:
        print(f"=== Running stage {args.stage} ===")
        {1: stage1, 2: stage2, 3: stage3, 4: stage4}[args.stage]()
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
