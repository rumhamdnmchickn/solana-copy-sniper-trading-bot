
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Ctrl {
    PauseAll,
    ResumeAll,
    AddWallet(String),
    PauseWallet{wallet:String, what:String},
    ExitPosition{mint:String},
    SetSlippage{wallet:String, bps:u64},
    SetTp{wallet:String, pct:f64},
    SetSl{wallet:String, pct:f64},
    SetRetries{wallet:String, n:u32},
    SetMaxPos{wallet:String, n:u32},
}

/// Parse very simple slash commands. Wire this to your telegram bot update stream.
pub fn parse_command(text: &str) -> Option<Ctrl> {
    let t = text.trim();
    let parts: Vec<&str> = t.split_whitespace().collect();
    match parts.get(0).copied().unwrap_or("") {
        "/pause_all" => Some(Ctrl::PauseAll),
        "/resume_all" => Some(Ctrl::ResumeAll),
        "/add_wallet" if parts.len()>=2 => Some(Ctrl::AddWallet(parts[1].to_string())),
        "/pause" if parts.len()>=3 => Some(Ctrl::PauseWallet{wallet:parts[1].into(), what:parts[2].into()}),
        "/exit" if parts.len()>=2 => Some(Ctrl::ExitPosition{mint:parts[1].into()}),
        "/slip" if parts.len()>=3 => Some(Ctrl::SetSlippage{wallet:parts[1].into(), bps:parts[2].parse().unwrap_or(250)}),
        "/tp" if parts.len()>=3 => Some(Ctrl::SetTp{wallet:parts[1].into(), pct:parts[2].parse().unwrap_or(5.0)}),
        "/sl" if parts.len()>=3 => Some(Ctrl::SetSl{wallet:parts[1].into(), pct:parts[2].parse().unwrap_or(12.0)}),
        "/retries" if parts.len()>=3 => Some(Ctrl::SetRetries{wallet:parts[1].into(), n:parts[2].parse().unwrap_or(3)}),
        "/maxpos" if parts.len()>=3 => Some(Ctrl::SetMaxPos{wallet:parts[1].into(), n:parts[2].parse().unwrap_or(3)}),
        _ => None
    }
}
