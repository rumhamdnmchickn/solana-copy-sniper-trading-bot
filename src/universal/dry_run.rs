use solana_sdk::signature::Signature;
use std::str::FromStr;

/// Result returned when in dry-run mode instead of a real blockchain signature.
#[derive(Debug, Clone)]
pub struct DryRunSignature;

impl DryRunSignature {
    pub fn mock() -> Signature {
        // A fake but valid-looking signature format.
        Signature::from_str("DryRun111111111111111111111111111111111111111").unwrap()
    }
}

/// Utility function that logs and returns a mock signature.
pub fn dry_run_send(label: &str) -> Signature {
    println!("[DRY RUN] Execution bypassed for: {}", label);
    DryRunSignature::mock()
}
