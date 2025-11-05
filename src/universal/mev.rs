
/// MEV protection placeholder. In production, integrate zeroslot & priority fee tuning.
pub struct MevProtection {
    pub tip_bps_cap: u64,
}

impl MevProtection {
    pub fn bound_tip_bps(&self, proposed_bps: u64) -> u64 {
        proposed_bps.min(self.tip_bps_cap)
    }
}
