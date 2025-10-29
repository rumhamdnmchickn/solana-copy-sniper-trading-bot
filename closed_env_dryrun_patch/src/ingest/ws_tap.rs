// src/ingest/ws_tap.rs
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static REC_PATH: Lazy<String> = Lazy::new(|| std::env::var("WS_RECORD_PATH").unwrap_or_else(|_| "./ws_feed.jsonl".into()));
static RECORD_WS: Lazy<bool> = Lazy::new(|| std::env::var("RECORD_WS").map(|s| s != "0").unwrap_or(true));
static FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| Mutex::new(None));

pub fn record_line(line: &str) {
    if !*RECORD_WS { return; }
    let mut guard = FILE.lock().unwrap();
    if guard.is_none() {
        if let Ok(f) = OpenOptions::new().create(true).append(true).open(&*REC_PATH) {
            *guard = Some(f);
        } else { return; }
    }
    if let Some(f) = guard.as_mut() {
        let _ = writeln!(f, "{}", line);
    }
}