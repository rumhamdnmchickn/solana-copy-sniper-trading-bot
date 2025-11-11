use super::MevConfig;

pub trait RawSubmit {
    type Sig;
    fn submit_via_rpc(&self, tx: &[u8]) -> anyhow::Result<Self::Sig>;
    fn submit_via_private(&self, tx: &[u8]) -> anyhow::Result<Self::Sig>;
    fn submit_via_jito_bundle(&self, txs: Vec<Vec<u8>>) -> anyhow::Result<Self::Sig>;
}

pub fn submit_with_protection<S: RawSubmit>(
    cfg: &MevConfig,
    submitter: &S,
    tx_bytes: &[u8],
) -> anyhow::Result<S::Sig> {
    match cfg.mode.as_str() {
        "private_rpc" => submitter.submit_via_private(tx_bytes),
        "jito_bundle" => submitter.submit_via_jito_bundle(vec![tx_bytes.to_vec()]),
        _ => submitter.submit_via_rpc(tx_bytes),
    }
}
