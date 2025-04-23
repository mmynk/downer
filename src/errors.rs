#[derive(Debug)]
pub enum Error {
    DirectoryNotFound(String),
    RequestFailed(reqwest::Error),
    IO(std::io::Error),
    InvalidHeaderValue(reqwest::header::InvalidHeaderValue),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::RequestFailed(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
