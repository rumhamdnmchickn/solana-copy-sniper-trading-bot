#!/usr/bin/env python3
import re
import subprocess
from pathlib import Path

RUST_FILE = Path("src/processor/sniper_bot.rs")

def run_cargo_check() -> str:
    """Run cargo check and return stderr as text."""
    proc = subprocess.run(
        ["cargo", "check"],
        capture_output=True,
        text=True,
    )
    return proc.stderr

def find_unmatched_brace_line(stderr: str) -> int | None:
    """
    Look for a line like:
      --> src/processor/sniper_bot.rs:5915:1
    after an 'unexpected closing delimiter: `}`' error.
    """
    # First make sure it's the right error
    if "unexpected closing delimiter: `}`" not in stderr:
        return None

    # Now find the file/line reference
    m = re.search(r"src/processor/sniper_bot\.rs:(\d+):\d+", stderr)
    if not m:
        return None
    return int(m.group(1))

def comment_out_line(path: Path, line_no: int) -> None:
    """
    Comment out the specified 1-based line in the file.
    Creates a .bak backup first.
    """
    text = path.read_text(encoding="utf-8").splitlines(keepends=True)

    if not (1 <= line_no <= len(text)):
        raise SystemExit(f"Line {line_no} out of range for {path}")

    # Make a backup
    backup = path.with_suffix(path.suffix + ".bak")
    backup.write_text("".join(text), encoding="utf-8")

    original = text[line_no - 1]
    # Replace that line with a comment (preserving indentation width)
    indent = len(original) - len(original.lstrip(" \t"))
    prefix = original[:indent]
    text[line_no - 1] = f"{prefix}// AUTO-FIX: commented out stray closing brace\n"

    path.write_text("".join(text), encoding="utf-8")
    print(f"Commented out line {line_no} in {path}")
    print(f"Backup written to {backup}")

def main() -> None:
    if not RUST_FILE.exists():
        raise SystemExit(f"Rust file not found: {RUST_FILE}")

    # Try multiple passes, stopping when there are no more brace errors
    for i in range(10):  # safety limit
        stderr = run_cargo_check()
        line_no = find_unmatched_brace_line(stderr)

        if line_no is None:
            print("No more 'unexpected closing delimiter: `}`' errors found.")
            break

        print(f"[pass {i+1}] Found unmatched '}}' reported at line {line_no}")
        comment_out_line(RUST_FILE, line_no)
    else:
        print("Hit loop limit; there may still be unmatched braces.")


if __name__ == "__main__":
    main()
