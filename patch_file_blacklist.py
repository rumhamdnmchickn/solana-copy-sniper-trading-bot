#!/usr/bin/env python3
"""
Patch sniper_bot.rs to add a file-based permanent token blacklist,
loaded from `blacklist.json`, and wire it into execute_buy.

Also (optionally) ensure `serde_json` is listed in Cargo.toml [dependencies].

Behavior:
- On first call to execute_buy, we:
  - load blacklist.json (if present and valid),
  - populate PERMANENT_FILE_BLACKLIST (DashMap<String, ()>),
  - log what happened.
- Every execute_buy then:
  - checks PERMANENT_FILE_BLACKLIST first,
  - THEN falls back to the existing BOUGHT_TOKENS_BLACKLIST
    (your "never rebuy after bought" logic).

blacklist.json is expected to be a JSON array of mint strings:
[
  "So11111111111111111111111111111111111111112",
  "Es9vMFrzaC...",
  "anotherMint..."
]
"""

from pathlib import Path


SNIPER_PATH = Path("src/processor/sniper_bot.rs")
CARGO_TOML_PATH = Path("Cargo.toml")


def patch_sniper_bot(content: str) -> str:
    original = content

    # ------------------------------------------------------------------
    # 1. Add PERMANENT_FILE_BLACKLIST + loader helper if missing
    # ------------------------------------------------------------------
    if "PERMANENT_FILE_BLACKLIST" not in content:
        # 1a. Add static to lazy_static block
        lazy_idx = content.find("lazy_static::lazy_static!")
        if lazy_idx == -1:
            raise SystemExit("Could not find lazy_static block in sniper_bot.rs")

        # Find BOUGHT_TOKENS_BLACKLIST line to insert after
        blacklist_decl = "BOUGHT_TOKENS_BLACKLIST"
        bl_idx = content.find(blacklist_decl, lazy_idx)
        if bl_idx == -1:
            raise SystemExit(
                "Could not find BOUGHT_TOKENS_BLACKLIST in lazy_static block"
            )

        line_end = content.find("\n", bl_idx)
        if line_end == -1:
            line_end = bl_idx + len(blacklist_decl)

        insert_pos = line_end + 1

        static_snippet = (
            "    // Static file-based permanent blacklist loaded from blacklist.json\n"
            "    static ref PERMANENT_FILE_BLACKLIST: Arc<DashMap<String, ()>> = Arc::new(DashMap::new());\n"
            "    static ref PERMANENT_FILE_BLACKLIST_LOADED: AtomicBool = AtomicBool::new(false);\n"
        )

        content = content[:insert_pos] + static_snippet + content[insert_pos:]

        # 1b. Insert helper function after lazy_static block
        # Find the end of the lazy_static block ("}\n\n" after it)
        lazy_end = content.find("}", lazy_idx)
        if lazy_end == -1:
            raise SystemExit("Could not find end of lazy_static block")

        # Move to newline after the closing brace
        lazy_end_newline = content.find("\n", lazy_end)
        if lazy_end_newline == -1:
            lazy_end_newline = lazy_end

        helper_snippet = r"""

fn load_permanent_blacklist_from_file(logger: &Logger) {
    // Load only once to avoid repeated file I/O and log spam
    if PERMANENT_FILE_BLACKLIST_LOADED.load(Ordering::SeqCst) {
        return;
    }
    PERMANENT_FILE_BLACKLIST_LOADED.store(true, Ordering::SeqCst);

    let path = "blacklist.json";

    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            logger.log(
                format!(
                    "No {} found or failed to read ({}); proceeding with empty permanent blacklist",
                    path, e
                )
                .yellow()
                .to_string(),
            );
            return;
        }
    };

    let parsed: Result<Vec<String>, _> = serde_json::from_str(&contents);
    let mints = match parsed {
        Ok(v) => v,
        Err(e) => {
            logger.log(
                format!(
                    "Failed to parse {} as JSON array of strings: {}",
                    path, e
                )
                .red()
                .to_string(),
            );
            return;
        }
    };

    for mint in mints {
        let trimmed = mint.trim();
        if !trimmed.is_empty() {
            PERMANENT_FILE_BLACKLIST.insert(trimmed.to_string(), ());
        }
    }

    logger.log(
        format!(
            "Loaded {} permanent blacklist entries from {}",
            PERMANENT_FILE_BLACKLIST.len(),
            path
        )
        .green()
        .to_string(),
    );
}

"""

        content = (
            content[: lazy_end_newline + 1] + helper_snippet + content[lazy_end_newline + 1 :]
        )

    # ------------------------------------------------------------------
    # 2. Wire new blacklist into execute_buy
    # ------------------------------------------------------------------
    if "file-based permanent blacklist" not in content:
        marker = 'pub async fn execute_buy'
        fn_idx = content.find(marker)
        if fn_idx == -1:
            raise SystemExit("Could not find execute_buy in sniper_bot.rs")

        # Find logger + start_time lines
        logger_line = 'let logger = Logger::new("[EXECUTE-BUY]'
        logger_idx = content.find(logger_line, fn_idx)
        if logger_idx == -1:
            raise SystemExit(
                "Could not find Logger init line in execute_buy; aborting patch"
            )

        # Find the 'let start_time = Instant::now();' line after logger
        start_line = "let start_time = Instant::now();"
        start_idx = content.find(start_line, logger_idx)
        if start_idx == -1:
            raise SystemExit(
                "Could not find `let start_time = Instant::now();` in execute_buy"
            )

        start_end = content.find("\n", start_idx)
        if start_end == -1:
            start_end = start_idx + len(start_line)

        insert_pos = start_end + 1

        new_block = r"""
    // Ensure file-based permanent blacklist is loaded once
    load_permanent_blacklist_from_file(&logger);

    // Check file-based permanent blacklist (static configuration)
    if PERMANENT_FILE_BLACKLIST.contains_key(&trade_info.mint) {
        logger.log(
            format!(
                "🚫 Token {} is in file-based permanent blacklist, skipping buy",
                trade_info.mint
            )
            .yellow()
            .to_string(),
        );
        return Err("Token is in file-based permanent blacklist".to_string());
    }

"""

        content = content[:insert_pos] + new_block + content[insert_pos:]

    # ------------------------------------------------------------------
    # 3. Make sure the new helper imports compile: serde_json is used
    #    via full path, and AtomicBool/Ordering are already imported
    #    at the top of sniper_bot.rs, so no extra use lines needed.
    # ------------------------------------------------------------------

    if content == original:
        print("sniper_bot.rs already appears to have file-based blacklist logic.")
        return original

    return content


def patch_cargo_toml(text: str) -> str:
    original = text

    if "serde_json" in text:
        print("Cargo.toml already mentions serde_json; not adding dependency.")
        return original

    dep_line = 'serde_json = "1.0"\n'

    idx = text.find("[dependencies]")
    if idx == -1:
        # No [dependencies] section; append one
        patched = text.rstrip() + "\n\n[dependencies]\n" + dep_line
    else:
        # Insert just after [dependencies] header line
        line_end = text.find("\n", idx)
        if line_end == -1:
            line_end = idx + len("[dependencies]")
        insert_pos = line_end + 1
        patched = text[:insert_pos] + dep_line + text[insert_pos:]

    if patched != original:
        print('Added `serde_json = "1.0"` to Cargo.toml [dependencies].')

    return patched


def main() -> None:
    # --- Patch sniper_bot.rs ---
    if not SNIPER_PATH.exists():
        raise SystemExit(
            f"Could not find {SNIPER_PATH}. Run this script from the repo root."
        )

    sniper_original = SNIPER_PATH.read_text()
    sniper_patched = patch_sniper_bot(sniper_original)

    if sniper_patched != sniper_original:
        backup = SNIPER_PATH.with_suffix(SNIPER_PATH.suffix + ".bak")
        backup.write_text(sniper_original)
        SNIPER_PATH.write_text(sniper_patched)
        print(f"Patched {SNIPER_PATH} (backup at {backup})")
    else:
        print("No changes written to sniper_bot.rs.")

    # --- Patch Cargo.toml (optional but recommended) ---
    if CARGO_TOML_PATH.exists():
        cargo_original = CARGO_TOML_PATH.read_text()
        cargo_patched = patch_cargo_toml(cargo_original)
        if cargo_patched != cargo_original:
            backup = CARGO_TOML_PATH.with_suffix(".toml.bak")
            backup.write_text(cargo_original)
            CARGO_TOML_PATH.write_text(cargo_patched)
            print(f"Patched {CARGO_TOML_PATH} (backup at {backup})")
        else:
            print("Cargo.toml unchanged.")
    else:
        print(
            "Cargo.toml not found; skipped serde_json dependency patch.\n"
            "If you see a compile error about `serde_json`, add it under [dependencies]:\n"
            'serde_json = "1.0"\n'
        )


if __name__ == "__main__":
    main()
