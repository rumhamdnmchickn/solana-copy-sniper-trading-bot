use once_cell::sync::Lazy;

pub mod types;
pub mod registry;
pub mod error;

pub use types::*;
pub use registry::*;
pub use error::*;

/// Global in-memory positions registry.
///
/// This is intentionally simple and synchronous for Phase 1. If we ever
/// need higher read concurrency, we can swap the internal lock in
/// `PositionsRegistry` from `Mutex` to `RwLock` without changing this API.
pub static GLOBAL_POSITIONS_REGISTRY: Lazy<PositionsRegistry> =
    Lazy::new(PositionsRegistry::new);
