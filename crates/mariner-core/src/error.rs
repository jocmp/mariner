use mariner_feeds::fetch::FetchError;
use mariner_feeds::finder::FindError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::database::Error),
    #[error(transparent)]
    Find(#[from] FindError),
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error("{url} returned HTTP {status}")]
    Status { url: String, status: u16 },
    #[error("no feed at {url}")]
    NoFeed { url: String },
    #[error("couldn't extract the article: {0}")]
    Extract(#[from] mercury_parser::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::Database(error.into())
    }
}
