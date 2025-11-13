#!/usr/bin/env python3
"""
Fix execute_buy_with_retry recursion in src/processor/sniper_bot.rs.

The previous retry patch replaced all `match execute_buy(` call sites
with `match execute_buy_with_retry(`, which accidentally turned the
wrapper into a recursive async fn (illegal without boxing).

This script:

- Locates the `execute_buy_with_retry` function definition.
- Only inside that function's body, replaces
      match execute_buy_with_retry(
  with
      match execute_buy(
- Leaves external call sites (that *should* call the wrapper) unchanged.

Idempotent: if no such recursive call is found in the wrapper body, no changes are made.
"""

from pathlib import Path


SNIPER_PATH = Path("src/processor/sniper_bot.rs")


def find_fn_block(text: str, fn_signature: str) -> tuple[int, int]:
    """
    Find the [start, end) range (in bytes) of the function body including braces
    whose definition line starts with `fn_signature` (or `pub async fn ...` etc.).

    Returns (brace_start, brace_end_plus_one).
    """
    start = text.find(fn_signature)
    if start == -1:
        raise SystemExit(f"Could not find function signature: {fn_signature}")

    brace_start = text.find("{", start)
    if brace_start == -1:
        raise SystemExit(f"Could not find opening '{{' for: {fn_signature}")

    depth = 0
    for i in range(brace_start, len(text)):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                # include the closing brace
                return brace_start, i + 1

    raise SystemExit(f"Could not find matching closing '}}' for: {fn_signature}")


def patch_wrapper(text: str) -> str:
    original = text

    # Look for our wrapper by its signature
    fn_sig = "pub async fn execute_buy_with_retry("
    if fn_sig not in text:
        print("execute_buy_with_retry not found; nothing to fix.")
        return text

    body_start, body_end = find_fn_block(text, fn_sig)

    body = text[body_start:body_end]

    # Only in the wrapper body, fix recursive calls:
    if "match execute_buy_with_retry(" not in body:
        print("No recursive calls to execute_buy_with_retry found in wrapper body.")
        return text

    new_body = body.replace("match execute_buy_with_retry(", "match execute_buy(")

    if new_body == body:
        print("Wrapper body unchanged (no replacements made).")
        return text

    patched = text[:body_start] + new_body + text[body_start + len(body):]

    print("Fixed recursive calls inside execute_buy_with_retry.")
    return patched


def main() -> None:
    if not SNIPER_PATH.exists():
        raise SystemExit(
            f"Could not find {SNIPER_PATH}. Run this script from the repo root."
        )

    original = SNIPER_PATH.read_text()
    patched = patch_wrapper(original)

    if patched == original:
        print("No changes written to sniper_bot.rs.")
        return

    backup = SNIPER_PATH.with_suffix(SNIPER_PATH.suffix + ".bak_retry_fix")
    backup.write_text(original)
    SNIPER_PATH.write_text(patched)
    print(f"Patched {SNIPER_PATH} (backup at {backup})")


if __name__ == "__main__":
    main()
