use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotLoggedIn,
    HttpError(String),
    IoError(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotLoggedIn => write!(f, "未登录，请先设置账号密码"),
            Error::HttpError(msg) => write!(f, "HTTP 错误: {}", msg),
            Error::IoError(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}
