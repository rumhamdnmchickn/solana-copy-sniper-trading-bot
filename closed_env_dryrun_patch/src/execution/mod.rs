// src/execution/mod.rs
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Simulate,
    Live,
}

impl ExecutionMode {
    pub fn from_env() -> Self {
        match std::env::var("EXECUTION_MODE").unwrap_or_else(|_| "DRY_RUN".into()).to_uppercase().as_str() {
            "LIVE" => ExecutionMode::Live,
            "SIMULATE" => ExecutionMode::Simulate,
            _ => ExecutionMode::DryRun,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExecResult {
    pub mode: &'static str,
    pub ok: bool,
    pub tx_sig_or_reason: String,
}

#[async_trait::async_trait]
pub trait TradeExecutor: Send + Sync {
    async fn execute(&self, tx_bytes: Vec<u8>) -> anyhow::Result<ExecResult>;
}

pub struct DryRunExecutor;

#[async_trait::async_trait]
impl TradeExecutor for DryRunExecutor {
    async fn execute(&self, _tx_bytes: Vec<u8>) -> anyhow::Result<ExecResult> {
        Ok(ExecResult { mode: "DRY_RUN", ok: true, tx_sig_or_reason: "not-sent".into() })
    }
}

pub struct SimExecutor {
    // pub rpc: solana_client::nonblocking::rpc_client::RpcClient,
}
#[async_trait::async_trait]
impl TradeExecutor for SimExecutor {
    async fn execute(&self, tx_bytes: Vec<u8>) -> anyhow::Result<ExecResult> {
        // TODO: build solana_sdk::transaction::VersionedTransaction from bytes
        // and call rpc.simulate_transaction() here. For now, just declare as simulated.
        let _ = tx_bytes;
        Ok(ExecResult { mode: "SIMULATE", ok: true, tx_sig_or_reason: "simulated-ok".into() })
    }
}

pub struct LiveExecutor {
    // pub rpc: solana_client::nonblocking::rpc_client::RpcClient,
}
#[async_trait::async_trait]
impl TradeExecutor for LiveExecutor {
    async fn execute(&self, _tx_bytes: Vec<u8>) -> anyhow::Result<ExecResult> {
        // TODO: send via rpc.send_transaction(). Keep skip_preflight=false.
        Ok(ExecResult { mode: "LIVE", ok: false, tx_sig_or_reason: "NOT-WIRED".into() })
    }
}