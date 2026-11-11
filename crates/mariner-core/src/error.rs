use mariner_feeds::finder::FindError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::database::Error),
    #[error(transparent)]
    Find(#[from] FindError),
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::Database(error.into())
    }
}
