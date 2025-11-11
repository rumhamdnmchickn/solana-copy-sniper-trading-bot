// src/library/rpc_failover.rs
// Simple RPC failover wrapper for the project's Anchor / solana_client RpcClient
//
// Usage:
//   let rpc_set = RpcClientSet::new(primary_url, backup_url_option);
//   let sig = rpc_set.send_and_confirm_transaction_with_failover(&tx)?;

use std::sync::Arc;
use std::time::Duration;

use anchor_client::solana_client::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;
use solana_sdk::signature::Signature;

/// Wrapper struct containing primary & optional backup RpcClient
#[derive(Clone)]
pub struct RpcClientSet {
    pub primary: Arc<RpcClient>,
    pub backup: Option<Arc<RpcClient>>,
}

impl RpcClientSet {
    /// Construct from primary URL and optional backup URL.
    pub fn new(primary_url: &str, backup_url: Option<&str>) -> Self {
        // Using same commitment/timeout pattern as project
        let timeout = Duration::from_secs(30);
        let primary = Arc::new(
            anchor_client::solana_client::rpc_client::RpcClient::new_with_timeout_and_commitment(
                primary_url.to_string(),
                timeout,
                solana_sdk::commitment_config::CommitmentConfig::processed(),
            ),
        );

        let backup_client = backup_url.map(|u| {
            Arc::new(
                anchor_client::solana_client::rpc_client::RpcClient::new_with_timeout_and_commitment(
                    u.to_string(),
                    timeout,
                    solana_sdk::commitment_config::CommitmentConfig::processed(),
                ),
            )
        });

        Self {
            primary,
            backup: backup_client,
        }
    }

    /// Try to send using primary; if that fails and a backup is configured, try the backup.
    /// Returns the successful tx signature or the last error.
    pub fn send_and_confirm_transaction_with_failover(
        &self,
        tx: &Transaction,
    ) -> Result<Signature, Box<dyn std::error::Error>> {
        // Try primary
        match self.primary.send_and_confirm_transaction(tx) {
            Ok(sig) => return Ok(sig),
            Err(e_primary) => {
                log::warn!("primary RPC send failed: {}", e_primary);
                // If backup exists, try it
                if let Some(backup) = &self.backup {
                    // small blocking backoff
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    match backup.send_and_confirm_transaction(tx) {
                        Ok(sig2) => {
                            log::warn!("backup RPC succeeded after primary failed");
                            return Ok(sig2);
                        }
                        Err(e_backup) => {
                            let err_msg = format!(
                                "both RPCs failed: primary: {} ; backup: {}",
                                e_primary, e_backup
                            );
                            return Err(Box::<dyn std::error::Error + Send + Sync>::from(err_msg));
                        }
                    }
                } else {
                    let err_msg = format!("primary RPC failed and no backup configured: {}", e_primary);
                    return Err(Box::<dyn std::error::Error + Send + Sync>::from(err_msg));
                }
            }
        }
    }
}
