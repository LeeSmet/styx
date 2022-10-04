use std::fmt;

pub mod ed25519;

/// Errors related to cryptographic operations.
#[derive(Debug)]
pub enum Error {
    /// The given data is not valid to construct a type.
    InvalidData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidData => f.pad("invalid data"),
        }
    }
}

impl std::error::Error for Error {}
