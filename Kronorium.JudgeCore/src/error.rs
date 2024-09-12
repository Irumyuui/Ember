use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("{0}")]
    Msg(String),

    #[error("Invalid language: {0}")]
    InvalidLanguage(String),

    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error(transparent)]
    Errno(#[from] nix::Error),
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Self::Msg(s.to_owned())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Msg(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn default_error_handler(err: &Error, output: &mut dyn std::io::Write) {
    use nu_ansi_term::Color::Red;
    writeln!(output, "{}: {}", Red.paint("[Error]"), err).ok();
}
