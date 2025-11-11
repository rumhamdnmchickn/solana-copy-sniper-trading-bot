// contents of file
use reqwest::{Client, Response};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use crate::common::config::RuntimeConfig;

#[derive(Clone)]
pub struct RpcClient {
    endpoints: Arc<Vec<String>>,
    // index of current endpoint
    cur_idx: Arc<Mutex<usize>>,
    client: Client,
    cfg: RuntimeConfig,
}

impl RpcClient {
    pub fn new(cfg: RuntimeConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(cfg.rpc_timeout())
            .build()
            .expect("reqwest client build");
        let endpoints = Arc::new(cfg.rpc_endpoints.clone());
        RpcClient {
            endpoints,
            cur_idx: Arc::new(Mutex::new(0)),
            client,
            cfg,
        }
    }

    fn current_endpoint(&self) -> String {
        let idx = *self.cur_idx.lock().unwrap();
        self.endpoints[idx].clone()
    }

    fn failover_to_next(&self) {
        let mut idx = self.cur_idx.lock().unwrap();
        *idx = (*idx + 1) % self.endpoints.len();
        log::warn!("RPC failover: switching to endpoint index {}", *idx);
    }

    async fn exponential_backoff_delay(&self, attempt: usize) {
        let base = self.cfg.rpc_backoff_base_ms as f64;
        let max = self.cfg.rpc_backoff_max_ms as f64;
        // simple exp backoff
        let mut delay = base * (2u64.pow(attempt as u32) as f64);
        if delay > max {
            delay = max;
        }
        sleep(Duration::from_millis(delay as u64)).await;
    }

    /// Perform a JSON-RPC POST request with failover and retry logic.
    /// payload is JSON body string, returns the successful Response or error.
    pub async fn post_rpc(&self, payload: String) -> Result<Response, reqwest::Error> {
        let endpoints_len = self.endpoints.len();
        if endpoints_len == 0 {
            return Err(reqwest::Error::new(
                reqwest::StatusCode::BAD_REQUEST,
                "no rpc endpoints configured",
            ));
        }

        // Try up to endpoints_len * cfg.rpc_retry_attempts times total, rotating endpoints on persistent failure.
        for endpoint_round in 0..endpoints_len {
            let cur_endpoint = {
                let idx = *self.cur_idx.lock().unwrap();
                self.endpoints[idx].clone()
            };

            for attempt in 0..self.cfg.rpc_retry_attempts {
                let url = cur_endpoint.clone();
                let res = self.client.post(&url).body(payload.clone()).send().await;
                match res {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            return Ok(resp);
                        } else {
                            log::warn!("RPC endpoint {} returned non-success status: {}", url, resp.status());
                        }
                    }
                    Err(e) => {
                        log::warn!("RPC request to {} failed on attempt {}: {}", url, attempt, e);
                    }
                }
                self.exponential_backoff_delay(attempt).await;
            }

            // After retry attempts for this endpoint, failover to next endpoint and continue
            self.failover_to_next();
        }

        Err(reqwest::Error::new(
            reqwest::StatusCode::REQUEST_TIMEOUT,
            "all rpc endpoints failed after retries",
        ))
    }
}
