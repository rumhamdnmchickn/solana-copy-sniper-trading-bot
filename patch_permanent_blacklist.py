#!/usr/bin/env python3
"""
Patch src/processor/sniper_bot.rs to implement a permanent token blacklist.

This script will:

1. Add a global static `BOUGHT_TOKENS_BLACKLIST: Arc<DashMap<String, u64>>`
   in the lazy_static block (if missing).

2. Add an early-return in `execute_buy` that skips any token already present
   in `BOUGHT_TOKENS_BLACKLIST` (if missing).

3. After every `BOUGHT_TOKEN_LIST.insert(...)` call, add logic that inserts
   the token into `BOUGHT_TOKENS_BLACKLIST` with a timestamp, and logs it
   (if missing).

The script is idempotent: if the relevant bits are already present, it will
leave the file unchanged.
"""

from pathlib import Path


def patch_content(content: str) -> str:
    import re

    original = content

    # ------------------------------------------------------------
    # 1. Add global BOUGHT_TOKENS_BLACKLIST static if missing
    # ------------------------------------------------------------
    if "BOUGHT_TOKENS_BLACKLIST" not in content:
        lazy_idx = content.find("lazy_static::lazy_static!")
        if lazy_idx == -1:
            raise SystemExit("Could not find lazy_static block; aborting blacklist patch.")

        # Prefer to insert near BOUGHT_TOKEN_LIST, if present
        idx = content.find("pub static ref BOUGHT_TOKEN_LIST", lazy_idx)
        if idx != -1:
            line_end = content.find("\n", idx)
            insert_pos = line_end + 1
        else:
            # Fallback: insert near the top of the lazy_static block
            insert_pos = content.find("{", lazy_idx) + 1

        static_snippet = (
            "    // Add: Permanent blacklist for tokens that have been bought before (never rebuy)\n"
            "    static ref BOUGHT_TOKENS_BLACKLIST: Arc<DashMap<String, u64>> = Arc::new(DashMap::new());\n"
        )
        content = content[:insert_pos] + static_snippet + content[insert_pos:]

    # ------------------------------------------------------------
    # 2. Add early-return check in execute_buy if blacklist present
    # ------------------------------------------------------------
    if "Token is blacklisted - previously bought" not in content:
        fn_marker = "pub async fn execute_buy"
        fn_idx = content.find(fn_marker)
        if fn_idx == -1:
            raise SystemExit("Could not find execute_buy function; aborting blacklist patch.")

        # Limit our search for the start_time line to the body of the function
        # (crude but adequate for this patch).
        brace_idx = content.find("{", fn_idx)
        if brace_idx == -1:
            raise SystemExit("Could not find execute_buy body; aborting blacklist patch.")

        # Search for the start_time initialization within some reasonable window
        search_end = content.find("}", brace_idx)
        if search_end == -1:
            search_end = brace_idx + 5000  # fallback window

        segment = content[brace_idx:search_end]
        start_line = "let start_time = Instant::now();"
        rel_idx = segment.find(start_line)
        if rel_idx == -1:
            raise SystemExit("Could not find `let start_time = Instant::now();` in execute_buy; aborting blacklist patch.")

        abs_idx = brace_idx + rel_idx
        line_end = abs_idx + len(start_line)
        newline_pos = content.find("\n", line_end)
        if newline_pos == -1:
            newline_pos = line_end

        insert_pos = newline_pos + 1

        check_snippet = (
            "    \n"
            "    // Check if this token is in the permanent blacklist (never rebuy)\n"
            "    if BOUGHT_TOKENS_BLACKLIST.contains_key(&trade_info.mint) {\n"
            "        logger.log(\n"
            "            format!(\"🚫 Token {} is blacklisted (previously bought), skipping buy\", trade_info.mint)\n"
            "                .yellow()\n"
            "                .to_string(),\n"
            "        );\n"
            "        return Err(\"Token is blacklisted - previously bought\".to_string());\n"
            "    }\n"
            "    \n"
        )

        content = content[:insert_pos] + check_snippet + content[insert_pos:]

    # ------------------------------------------------------------
    # 3. After each BOUGHT_TOKEN_LIST.insert, add blacklist insertion
    # ------------------------------------------------------------
    if "Added {} to permanent blacklist" not in content:
        lines = content.splitlines(keepends=True)
        out_lines = []
        i = 0

        while i < len(lines):
            line = lines[i]
            out_lines.append(line)

            if "BOUGHT_TOKEN_LIST.insert" in line:
                # Look ahead to see if we already have a BOUGHT_TOKENS_BLACKLIST insert nearby
                lookahead = "".join(lines[i + 1 : i + 8])
                if "BOUGHT_TOKENS_BLACKLIST" not in lookahead:
                    # Match indentation of the current line
                    indent = line[: len(line) - len(line.lstrip(" "))]
                    snippet = (
                        f"{indent}// Add to permanent blacklist (never rebuy this token)\n"
                        f"{indent}let timestamp = std::time::SystemTime::now()\n"
                        f"{indent}    .duration_since(std::time::UNIX_EPOCH)\n"
                        f"{indent}    .unwrap_or_default()\n"
                        f"{indent}    .as_secs();\n"
                        f"{indent}BOUGHT_TOKENS_BLACKLIST.insert(trade_info.mint.clone(), timestamp);\n"
                        f"{indent}logger.log(format!(\"🚫 Added {{}} to permanent blacklist\", trade_info.mint));\n"
                    )
                    out_lines.append(snippet)

            i += 1

        content = "".join(out_lines)

    if content == original:
        # No changes made; likely already patched
        return original

    return content


def main() -> None:
    path = Path("src/processor/sniper_bot.rs")
    if not path.exists():
        raise SystemExit(f"File not found: {path} (run this from the repo root).")

    original = path.read_text()

    patched = patch_content(original)

    if patched == original:
        print("No changes made: sniper_bot.rs already appears to have permanent blacklist logic.")
        return

    backup_path = path.with_suffix(path.suffix + ".bak")
    backup_path.write_text(original)
    path.write_text(patched)

    print(f"Patched {path}")
    print(f"Backup written to {backup_path}")


if __name__ == "__main__":
    main()
