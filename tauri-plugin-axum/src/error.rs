use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("require x-method header")]
    Method,
    #[error("require x-uri header")]
    Uri,
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] axum::http::Error),
    #[error(transparent)]
    Axum(#[from] axum::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("request canceled")]
    Canceled,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
