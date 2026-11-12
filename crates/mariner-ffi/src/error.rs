//! Errors as the apps see them.

use mariner_core::Error;
use mariner_feeds::finder::FindError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MarinerError {
    /// The database failed; nothing the user can fix by retrying.
    #[error("{reason}")]
    Database { reason: String },
    /// No response: offline, timed out, or the URL is no good.
    #[error("{reason}")]
    Network { reason: String },
    /// A response, but not a successful one.
    #[error("{url} returned HTTP {status}")]
    Status { url: String, status: u16 },
    /// The page came back but its article couldn't be read.
    #[error("{reason}")]
    Extract { reason: String },
    /// Condensing failed: the base URL isn't a URL.
    #[error("{reason}")]
    Condense { reason: String },
}

impl From<Error> for MarinerError {
    fn from(error: Error) -> Self {
        let reason = error.to_string();
        match error {
            Error::Database(_) => MarinerError::Database { reason },
            Error::Fetch(_) => MarinerError::Network { reason },
            Error::Find(FindError::Status { url, status }) | Error::Status { url, status } => {
                MarinerError::Status { url, status }
            }
            Error::Find(_) => MarinerError::Network { reason },
            Error::Extract(_) => MarinerError::Extract { reason },
        }
    }
}

impl From<mariner_core::database::Error> for MarinerError {
    fn from(error: mariner_core::database::Error) -> Self {
        MarinerError::Database {
            reason: error.to_string(),
        }
    }
}
