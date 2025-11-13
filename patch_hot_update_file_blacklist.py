#!/usr/bin/env python3
"""
Patch src/processor/sniper_bot.rs to add hot-update helpers for the
file-based permanent blacklist.

New functions (public, but allowed to be unused):

    #[allow(dead_code)]
    pub fn add_mint_to_file_blacklist(mint: &str, logger: &Logger) -> Result<(), String>

    #[allow(dead_code)]
    pub fn remove_mint_from_file_blacklist(mint: &str, logger: &Logger) -> Result<(), String>

Both:
- Ensure the current blacklist is loaded from `blacklist.json`
- Update PERMANENT_FILE_BLACKLIST in-memory
- Persist the updated set back to blacklist.json as a JSON array of strings

Assumes you already applied the previous patch that introduced:
- PERMANENT_FILE_BLACKLIST
- PERMANENT_FILE_BLACKLIST_LOADED
- fn load_permanent_blacklist_from_file(logger: &Logger)
"""

from pathlib import Path

SNIPER_PATH = Path("src/processor/sniper_bot.rs")


def find_function_block(text: str, fn_signature: str) -> int:
    """
    Find the end index (position after closing brace) of the function
    whose definition line starts with the given `fn_signature`.
    """
    start = text.find(fn_signature)
    if start == -1:
        raise SystemExit(f"Could not find function signature: {fn_signature}")

    # Find the first '{' after the signature
    brace_start = text.find("{", start)
    if brace_start == -1:
        raise SystemExit(f"Could not find '{{' for function: {fn_signature}")

    depth = 0
    for i in range(brace_start, len(text)):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                # Move to the character after this closing brace
                return i + 1

    raise SystemExit(f"Could not find matching closing brace for: {fn_signature}")


def patch_sniper_bot(text: str) -> str:
    original = text

    # If we've already added the helpers, do nothing
    if "fn add_mint_to_file_blacklist" in text:
        print("sniper_bot.rs already has hot-update helpers; no changes made.")
        return text

    # Ensure the loader function exists so we know where to anchor
    fn_sig = "fn load_permanent_blacklist_from_file(logger: &Logger)"
    end_idx = find_function_block(text, fn_sig)

    # Insert after the loader's closing brace, with a blank line before/after
    insert_pos = end_idx
    if insert_pos < len(text) and text[insert_pos] != "\n":
        # Try to be neat: align with following newline
        insert_pos = text.find("\n", insert_pos)
        if insert_pos == -1:
            insert_pos = end_idx
        else:
            insert_pos += 1

    helpers = r"""

#[allow(dead_code)]
pub fn add_mint_to_file_blacklist(mint: &str, logger: &Logger) -> Result<(), String> {
    // Ensure file-based blacklist is loaded so we merge, not overwrite
    load_permanent_blacklist_from_file(logger);

    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return Err("Mint cannot be empty".to_string());
    }

    PERMANENT_FILE_BLACKLIST.insert(trimmed.to_string(), ());

    // Collect all current mints into a Vec<String>
    let mints: Vec<String> = PERMANENT_FILE_BLACKLIST
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    let json = serde_json::to_string_pretty(&mints)
        .map_err(|e| format!("Failed to serialize blacklist.json: {}", e))?;

    if let Err(e) = std::fs::write("blacklist.json", json) {
        logger.log(
            format!("Failed to write blacklist.json: {}", e)
                .red()
                .to_string(),
        );
        return Err(format!("Failed to write blacklist.json: {}", e));
    }

    logger.log(
        format!(
            "✅ Added {} to file-based permanent blacklist and persisted to blacklist.json",
            trimmed
        )
        .green()
        .to_string(),
    );

    Ok(())
}

#[allow(dead_code)]
pub fn remove_mint_from_file_blacklist(mint: &str, logger: &Logger) -> Result<(), String> {
    // Ensure file-based blacklist is loaded so state is consistent
    load_permanent_blacklist_from_file(logger);

    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return Err("Mint cannot be empty".to_string());
    }

    let existed = PERMANENT_FILE_BLACKLIST.remove(trimmed).is_some();

    if !existed {
        logger.log(
            format!(
                "Mint {} was not present in file-based permanent blacklist",
                trimmed
            )
            .yellow()
            .to_string(),
        );
    }

    // Persist the updated set back to blacklist.json
    let mints: Vec<String> = PERMANENT_FILE_BLACKLIST
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    let json = serde_json::to_string_pretty(&mints)
        .map_err(|e| format!("Failed to serialize blacklist.json: {}", e))?;

    if let Err(e) = std::fs::write("blacklist.json", json) {
        logger.log(
            format!("Failed to write blacklist.json: {}", e)
                .red()
                .to_string(),
        );
        return Err(format!("Failed to write blacklist.json: {}", e));
    }

    logger.log(
        format!(
            "✅ Removed {} from file-based permanent blacklist and updated blacklist.json",
            trimmed
        )
        .green()
        .to_string(),
    );

    Ok(())
}

"""

    patched = text[:insert_pos] + helpers + text[insert_pos:]

    if patched == original:
        print("No changes applied to sniper_bot.rs (patch ended up identical).")
    else:
        print("Hot-update helpers inserted into sniper_bot.rs.")

    return patched


def main() -> None:
    if not SNIPER_PATH.exists():
        raise SystemExit(
            f"Could not find {SNIPER_PATH}. Run this script from the repo root."
        )

    original = SNIPER_PATH.read_text()
    patched = patch_sniper_bot(original)

    if patched != original:
        backup = SNIPER_PATH.with_suffix(SNIPER_PATH.suffix + ".bak_hotupdate")
        backup.write_text(original)
        SNIPER_PATH.write_text(patched)
        print(f"Patched {SNIPER_PATH} (backup at {backup})")
    else:
        print("sniper_bot.rs left unchanged.")


if __name__ == "__main__":
    main()
