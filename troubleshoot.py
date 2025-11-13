#!/usr/bin/env python3
import argparse, re, shutil
from pathlib import Path

def rust_iter_tokens(line):
    """Yield only real brace tokens, skipping strings and comments."""
    i, n = 0, len(line)
    in_sl_comment = False
    in_ml_comment = False
    in_str = False
    str_delim = None
    while i < n:
        ch = line[i]
        nxt = line[i+1] if i+1 < n else ''
        if in_sl_comment:
            break
        if in_ml_comment:
            if ch == '*' and nxt == '/':
                in_ml_comment = False; i += 2; continue
            i += 1; continue
        if in_str:
            if ch == '\\' and i+1 < n:
                i += 2; continue
            if ch == str_delim:
                in_str = False; str_delim = None
            i += 1; continue
        # not in comment/string
        if ch == '/' and nxt == '/':
            in_sl_comment = True; i += 2; continue
        if ch == '/' and nxt == '*':
            in_ml_comment = True; i += 2; continue
        if ch in ('"', "'"):
            in_str = True; str_delim = ch; i += 1; continue
        if ch in '{}()[]':
            yield ch
        i += 1

def scan_file(path: Path):
    lines = path.read_text(encoding='utf-8').splitlines()
    depth = 0
    events = []
    bad = []
    for ln, line in enumerate(lines, start=1):
        for tok in rust_iter_tokens(line):
            if tok in '([{':
                depth += 1
                events.append((ln, tok, depth))
            elif tok in ')]}':
                if depth == 0:
                    bad.append((ln, f"Unmatched closing {tok}"))
                depth -= 1
                events.append((ln, tok, depth))
    return lines, events, bad, depth

def show_hotspots(lines, hotspots):
    for ln in hotspots:
        a = max(1, ln - 6); b = min(len(lines), ln + 6)
        print(f"\n--- context {a}..{b} (focus {ln}) ---")
        for i in range(a, b+1):
            mark = '>>' if i == ln else '  '
            print(f"{mark} {i:5}: {lines[i-1]}")

def replace_range(path: Path, start: int, end: int, payload_path: Path):
    src = path.read_text(encoding='utf-8').splitlines()
    payload = payload_path.read_text(encoding='utf-8').splitlines()
    bak = path.with_suffix(path.suffix + ".bak")
    shutil.copy2(path, bak)
    new = src[:start-1] + payload + src[end:]
    path.write_text('\n'.join(new) + '\n', encoding='utf-8')
    print(f"Replaced {start}..{end} (backup: {bak})")

def main():
    p = argparse.ArgumentParser()
    p.add_argument("file", type=Path)
    p.add_argument("--scan", action="store_true", help="Rust-aware delimiter scan")
    p.add_argument("--replace-range", nargs=2, type=int, metavar=("START","END"))
    p.add_argument("--with-file", type=Path)
    args = p.parse_args()

    if args.replace_range:
        if not args.with_file:
            print("--replace-range requires --with-file"); return
        replace_range(args.file, args.replace_range[0], args.replace_range[1], args.with_file)
        return

    if args.scan:
        lines, events, bad, depth = scan_file(args.file)
        print(f"scan: closing-errors={len(bad)}, final_depth={depth}")
        for ln, msg in bad:
            print(f" - {ln}: {msg}")
        # highlight suspect cargo hints if present
        hints = [103, 181, 189, 280, 5934]
        show_hotspots(lines, hints)
        # Also show where depth goes negative or never returns to 0
        negs = [ln for ln, tok, d in events if d < 0]
        if negs:
            print("\nDepth went negative at:", negs[:10])
    else:
        print("Nothing to do. Try --scan or --replace-range.")
if __name__ == "__main__":
    main()
