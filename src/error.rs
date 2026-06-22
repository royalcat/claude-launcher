use std::fmt;

#[derive(Debug)]
pub struct ConfigCorruptError {
    pub path: String,
    pub cause: String,
}

impl fmt::Display for ConfigCorruptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Config file at {} is not valid JSON: {}", self.path, self.cause)
    }
}

impl std::error::Error for ConfigCorruptError {}

#[derive(Debug)]
pub struct ConfigAccessError {
    pub path: String,
    pub cause: String,
}

impl fmt::Display for ConfigAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cannot access config file at {}: {}", self.path, self.cause)
    }
}

impl std::error::Error for ConfigAccessError {}

#[derive(Debug)]
pub enum AppError {
    Corrupt(ConfigCorruptError),
    Access(ConfigAccessError),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Corrupt(e) => write!(f, "{e}"),
            AppError::Access(e) => write!(f, "{e}"),
            AppError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ConfigCorruptError> for AppError {
    fn from(e: ConfigCorruptError) -> Self {
        AppError::Corrupt(e)
    }
}

impl From<ConfigAccessError> for AppError {
    fn from(e: ConfigAccessError) -> Self {
        AppError::Access(e)
    }
}
