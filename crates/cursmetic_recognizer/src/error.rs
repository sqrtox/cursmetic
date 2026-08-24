use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Windows(#[from] windows::core::Error),
    #[error("Path {0} not exists")]
    PathNotExists(PathBuf),
}

impl From<windows::Win32::Foundation::WIN32_ERROR> for Error {
    fn from(value: windows::Win32::Foundation::WIN32_ERROR) -> Self {
        Self::from(windows::core::Error::from(value))
    }
}
