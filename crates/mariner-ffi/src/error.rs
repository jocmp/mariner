//! Errors as the apps see them.

use mariner_core::Error;
use mariner_feeds::finder::FindError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MarinerError {
    /// The database failed; nothing the user can fix by retrying.
    #[error("{message}")]
    Database { message: String },
    /// No response: offline, timed out, or the URL is no good.
    #[error("{message}")]
    Network { message: String },
    /// A response, but not a successful one.
    #[error("{url} returned HTTP {status}")]
    Status { url: String, status: u16 },
    /// The page came back but its article couldn't be read.
    #[error("{message}")]
    Extract { message: String },
}

impl From<Error> for MarinerError {
    fn from(error: Error) -> Self {
        let message = error.to_string();
        match error {
            Error::Database(_) => MarinerError::Database { message },
            Error::Fetch(_) => MarinerError::Network { message },
            Error::Find(FindError::Status { url, status }) | Error::Status { url, status } => {
                MarinerError::Status { url, status }
            }
            Error::Find(_) => MarinerError::Network { message },
            Error::Extract(_) => MarinerError::Extract { message },
        }
    }
}

impl From<mariner_core::database::Error> for MarinerError {
    fn from(error: mariner_core::database::Error) -> Self {
        MarinerError::Database {
            message: error.to_string(),
        }
    }
}
