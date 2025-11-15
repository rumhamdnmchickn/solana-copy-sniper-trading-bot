use std::fmt;

/// Errors that can occur when interacting with the positions registry.
#[derive(Debug)]
pub enum PositionError {
    /// There is already an open position for the given (wallet, mint) pair.
    AlreadyOpen(String, String),
    /// There is no open position for the given (wallet, mint) pair.
    NotOpen(String, String),
    /// A generic internal error.
    Internal(String),
}

impl fmt::Display for PositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionError::AlreadyOpen(wallet, mint) => {
                write!(f, "Position already open for wallet={}, mint={}", wallet, mint)
            }
            PositionError::NotOpen(wallet, mint) => {
                write!(f, "No open position for wallet={}, mint={}", wallet, mint)
            }
            PositionError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for PositionError {}
