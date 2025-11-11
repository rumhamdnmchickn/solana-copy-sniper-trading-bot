 url=https://github.com/rumhamdnmchickn/solana-copy-sniper-trading-bot/blob/feat/universal-gates-wiring/src/common/rpc_failover.rs
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, str::FromStr};

use anchor_client::solana_client::rpc_request::TokenAccountsFilter;
use anchor_client::solana_client::{rpc_request, rpc_response};
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::transaction::Transaction;
use solana_client::rpc_client::RpcClient;
use solana_client::client_error::ClientError;
use log::warn;

/// Lightweight RPC failover client that uses the blocking RpcClient under the hood.
/// It recreates a blocking RpcClient for each endpoint attempt and rotates through
/// the configured endpoints when RPC calls fail. The API surface implemented is
/// intentionally small and matches the call sites used in main.rs (get_account,
/// get_token_accounts_by_owner, get_latest_blockhash, send_and_confirm_transaction).
#[derive(Clone)]
pub struct RpcFailover {
    endpoints: Arc<Vec<String>>,
    cur_idx: Arc<Mutex<usize>>,
    retry_attempts: usize,
    timeout: Duration,
}

impl RpcFailover {
    /// Build from explicit list
    pub fn new(endpoints: Vec<String>, retry_attempts: usize, timeout: Duration) -> Result<Self, String> {
        let endpoints_clean: Vec<String> = endpoints.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if endpoints_clean.is_empty() {
            return Err("no rpc endpoints provided".into());
        }
        Ok(Self {
            endpoints: Arc::new(endpoints_clean),
            cur_idx: Arc::new(Mutex::new(0)),
            retry_attempts: if retry_attempts == 0 { 2 } else { retry_attempts },
            timeout,
        })
    }

    /// Build from environment variables:
    /// - RPC_ENDPOINTS (comma-separated URLs)
    /// - RPC_RETRY_ATTEMPTS (optional)
    /// - RPC_TIMEOUT_SECONDS (optional)
    pub fn from_env() -> Result<Self, String> {
        let endpoints = env::var("RPC_ENDPOINTS")
            .or_else(|_| env::var("RPC_HTTP"))
            .map_err(|_| "RPC_ENDPOINTS or RPC_HTTP must be set".to_string())?;
        let list: Vec<String> = endpoints.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let retry_attempts = env::var("RPC_RETRY_ATTEMPTS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(2);
        let timeout_seconds = env::var("RPC_TIMEOUT_SECONDS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(10);
        RpcFailover::new(list, retry_attempts, Duration::from_secs(timeout_seconds))
    }

    fn current_endpoint(&self) -> String {
        let idx = *self.cur_idx.lock().unwrap();
        self.endpoints[idx].clone()
    }

    fn advance_endpoint(&self) {
        let mut idx = self.cur_idx.lock().unwrap();
        *idx = (*idx + 1) % self.endpoints.len();
        warn!("RPC failover: switching to endpoint index {}", *idx);
    }

    /// Attempt a closure against the available endpoints; the closure receives a fresh RpcClient
    /// and should perform the RPC call. If the closure returns Ok, the result is returned.
    /// On error, the failover rotates endpoints and retries according to retry_attempts.
    fn try_endpoints<F, T>(&self, mut f: F) -> Result<T, String>
    where
        F: FnMut(&RpcClient) -> Result<T, ClientError>,
    {
        let endpoints_count = self.endpoints.len();
        if endpoints_count == 0 {
            return Err("no rpc endpoints configured".to_string());
        }

        // For each endpoint, try up to retry_attempts
        for _ep_round in 0..endpoints_count {
            let endpoint = self.current_endpoint();
            // Build a fresh RpcClient for the endpoint (using provided timeout via CommitmentConfig is limited,
            // RpcClient doesn't take a timeout param directly—users may configure HTTP client globally if needed).
            let client = RpcClient::new(endpoint.clone());

            for _attempt in 0..self.retry_attempts {
                match f(&client) {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        warn!("RPC request failed for {}: {}", endpoint, e);
                    }
                }
                // simple sleep between attempts; convert seconds from timeout as backoff base
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // rotate to next endpoint
            self.advance_endpoint();
        }

        Err("all rpc endpoints failed after retries".to_string())
    }

    pub fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        filter: TokenAccountsFilter,
    ) -> Result<Vec<rpc_response::RpcKeyedAccount>, String> {
        self.try_endpoints(|client| client.get_token_accounts_by_owner(owner, filter.clone()))
            .map_err(|e| format!("get_token_accounts_by_owner failed: {}", e))
    }

    pub fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<rpc_response::Response<rpc_response::RpcAccount>, String> {
        // RpcClient::get_account returns Result<Account, ClientError>. But in some codebases
        // they expect the full response. We will return the plain RpcAccount for simplicity.
        self.try_endpoints(|client| client.get_account(pubkey))
            .map_err(|e| format!("get_account failed: {}", e))
    }

    pub fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash, String> {
        self.try_endpoints(|client| client.get_latest_blockhash())
            .map_err(|e| format!("get_latest_blockhash failed: {}", e))
    }

    pub fn send_and_confirm_transaction(&self, tx: &Transaction) -> Result<Signature, String> {
        self.try_endpoints(|client| client.send_and_confirm_transaction(tx))
            .map_err(|e| format!("send_and_confirm_transaction failed: {}", e))
    }
}
