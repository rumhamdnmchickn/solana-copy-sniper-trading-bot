// src/bin/replay_ws.rs
use std::{fs::File, io::{BufRead, BufReader}};
use std::sync::Arc;
use dashmap::DashMap;
use your_crate::ingest::birdeye_ws::{LiveTokenMetrics, MetricsMap};
use your_crate::ingest::birdeye_ws::handle_payload as handle;

fn metrics() -> MetricsMap { Arc::new(DashMap::new()) }

fn main() {
    let path = std::env::var("REPLAY_WS_PATH").unwrap_or_else(|_| "./ws_feed.jsonl".into());
    let file = File::open(&path).expect("open replay file");
    let reader = BufReader::new(file);
    let m = metrics();
    let roll = Arc::new(DashMap::new());
    for line in reader.lines() {
        if let Ok(txt) = line {
            let _ = handle(&txt, &m, &roll);
        }
    }
    println!("replay complete; {} tokens cached", m.len());
}