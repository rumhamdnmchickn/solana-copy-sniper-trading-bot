#!/usr/bin/env python3
# Cargo Progress — summarize & diff cargo check errors across runs.
#
# Usage (from your project root):
#   python3 cargo_progress.py                # run cargo check, summarize, save baseline
#   python3 cargo_progress.py --no-save      # run without updating baseline
#   python3 cargo_progress.py --compare prev.json  # diff against a specific file
#
# It writes/reads `.cargo_check_progress.json` in the current directory by default.

import argparse
import json
import subprocess
import sys
import hashlib
from pathlib import Path
from collections import Counter
from datetime import datetime

DEFAULT_BASELINE = ".cargo_check_progress.json"


def run_cargo_check(extra_args=None):
    cmd = ["cargo", "check", "--message-format=json"]
    if extra_args:
        cmd += extra_args
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    lines = proc.stdout.splitlines()
    messages = []
    for line in lines:
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("reason") == "compiler-message":
            m = obj.get("message", {})
            messages.append(m)
    return messages, proc.returncode


def key_for_message(m):
    code = (m.get("code") or {}).get("code") or "nocode"
    level = m.get("level", "")
    render = (m.get("rendered") or "").splitlines()[0:2]
    render = " | ".join(render).strip()
    file_line = ""
    for s in m.get("spans") or []:
        if s.get("is_primary"):
            file_line = f"{s.get('file_name')}:{s.get('line_start')}:{s.get('column_start')}"
            break
    raw = f"{level}|{code}|{file_line}|{render}"
    return hashlib.sha1(raw.encode("utf-8")).hexdigest()


def summarize(messages):
    errors = [m for m in messages if m.get("level") == "error"]
    warns = [m for m in messages if m.get("level") == "warning"]
    codes = Counter(((m.get("code") or {}).get("code") or "nocode") for m in errors)
    files = Counter()
    for m in errors:
        for s in m.get("spans") or []:
            if s.get("is_primary"):
                files[s.get("file_name")] += 1
                break
    return {
        "total_errors": len(errors),
        "total_warnings": len(warns),
        "by_code": dict(codes),
        "by_file": dict(files),
    }


def load_baseline(path):
    p = Path(path)
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except Exception:
        return None


def save_run(path, data):
    Path(path).write_text(json.dumps(data, indent=2), encoding="utf-8")


def pretty_counter(dct, top=10):
    items = sorted(dct.items(), key=lambda kv: kv[1], reverse=True)[:top]
    if not items:
        return "  (none)"
    return "\n".join([f"  {k:>12} × {v}" for k, v in items])


def main():
    ap = argparse.ArgumentParser(
        description="Summarize and diff cargo check errors across runs."
    )
    ap.add_argument(
        "--no-save", action="store_true", help="do not update the baseline file"
    )
    ap.add_argument(
        "--compare",
        type=str,
        help="compare against a specific baseline JSON file",
    )
    ap.add_argument(
        "--baseline",
        type=str,
        default=DEFAULT_BASELINE,
        help="baseline file path",
    )
    ap.add_argument("extra", nargs="*", help="extra args passed to `cargo check`")
    args = ap.parse_args()

    msgs, rc = run_cargo_check(args.extra)
    now = datetime.utcnow().isoformat() + "Z"
    keys = [key_for_message(m) for m in msgs if m.get("level") == "error"]
    cur = {
        "timestamp": now,
        "return_code": rc,
        "messages": msgs,
        "error_keys": keys,
    }

    base_path = args.compare or args.baseline
    baseline = load_baseline(base_path)

    s = summarize(msgs)
    print("\n=== cargo check summary ===")
    print(f"errors:   {s['total_errors']}")
    print(f"warnings: {s['total_warnings']}")
    print("\nTop error codes:")
    print(pretty_counter(s["by_code"]))
    print("\nTop files with errors:")
    print(pretty_counter(s["by_file"]))

    if baseline:
        prev = set(baseline.get("error_keys", []))
        cur_set = set(keys)
        new_errs = cur_set - prev
        fixed = prev - cur_set
        same = len(prev & cur_set)

        print("\n=== diff vs previous run ===")
        print(f"new errors:   {len(new_errs)}")
        print(f"resolved:     {len(fixed)}")
        print(f"unchanged:    {same}")
        denom = len(prev) if prev else 1
        progress = (len(fixed) / denom) * 100.0
        print(f"\nprogress since last run: {progress:.1f}% of prior errors resolved")

        if new_errs:
            print("\nNew error examples:")
            shown = 0
            for k in new_errs:
                for m in msgs:
                    if key_for_message(m) == k:
                        loc = ""
                        for sp in m.get("spans") or []:
                            if sp.get("is_primary"):
                                loc = f"{sp.get('file_name')}:{sp.get('line_start')}:{sp.get('column_start')}"
                                break
                        code = (m.get("code") or {}).get("code") or "nocode"
                        headline = (m.get("rendered") or "").splitlines()[0].strip()
                        print(f"  [{code}] {loc}  {headline}")
                        shown += 1
                        if shown >= 5:
                            break
                if shown >= 5:
                    break
    else:
        print("\n(no previous baseline to diff against)")

    if not args.no_save:
        save_run(args.baseline, cur)
        print(f"\nSaved baseline to {args.baseline} at {now}")
    else:
        print("\n(Did not save baseline; use without --no-save to persist)")


if __name__ == "__main__":
    main()
