#!/usr/bin/env python3
"""
Patch src/processor/sniper_bot.rs to add a basic universal gates stub.

- Adds a helper function:
    fn passes_universal_gates(...)
- Wires it into execute_buy() right after the permanent blacklist check.

The stub always returns true, so behavior is unchanged. All new code is
behind the `universal_gates` feature flag to keep builds clean when the
feature is disabled.
"""

from pathlib import Path


def insert_helper(content: str) -> str:
    """Insert the passes_universal_gates helper before execute_buy()."""
    if "fn passes_universal_gates" in content:
        # Already patched
        return content

    marker = "/// Execute buy operation based on detected transaction"
    if marker not in content:
        raise SystemExit("Could not find execute_buy comment marker; aborting patch.")

    idx = content.index(marker)

    helper = r"""
#[cfg(feature = "universal_gates")]
fn passes_universal_gates(
    trade_info: &transaction_parser::TradeInfoFromToken,
) -> bool {
    // TODO: implement real universal gates (liquidity, market cap, volatility, etc.).
    // Stub returns true for now so behavior is unchanged while wiring is being built.
    true
}

"""

    return content[:idx] + helper + content[idx:]


def insert_gate_call(content: str) -> str:
    """Insert the call to passes_universal_gates inside execute_buy()."""
    if "Universal gates blocked buy" in content:
        # Already patched
        return content

    lines = content.splitlines(keepends=True)

    # Find the "Token is blacklisted" line and insert after the closing brace.
    target_substring = 'Token is blacklisted - previously bought'
    idx = None
    for i, line in enumerate(lines):
        if target_substring in line:
            idx = i
            break

    if idx is None:
        raise SystemExit("Could not find blacklist error line; aborting patch.")

    # line i is return Err(...), line i+1 is '    }'
    # Insert AFTER that closing brace.
    insert_pos = idx + 2

    gate_block = r"""
    #[cfg(feature = "universal_gates")]
    {
        if !passes_universal_gates(&trade_info) {
            logger.log(format!("🚫 Universal gates blocked buy for token {} (stubbed)", trade_info.mint));
            return Err("Universal gates blocked buy (stub)".to_string());
        }
    }

"""

    lines.insert(insert_pos, gate_block)
    return "".join(lines)


def main():
    # Adjust this path if sniper_bot.rs lives somewhere else.
    path = Path("src/processor/sniper_bot.rs")
    if not path.exists():
        raise SystemExit(f"File not found: {path}")

    original = path.read_text()

    patched = insert_helper(original)
    patched = insert_gate_call(patched)

    if patched == original:
        print("No changes made (file already patched?).")
        return

    backup_path = path.with_suffix(path.suffix + ".bak")
    backup_path.write_text(original)
    path.write_text(patched)

    print(f"Patched {path}")
    print(f"Backup written to {backup_path}")


if __name__ == "__main__":
    main()
