use std::io::{Error, ErrorKind};

pub fn config(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}

pub fn data(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}

pub fn external(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Other, message.into())
}

pub fn missing_env(name: &str, context: impl AsRef<str>) -> Error {
    config(format!(
        "missing required environment variable `{name}` ({})",
        context.as_ref()
    ))
}

pub fn missing_config(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::NotFound, message.into())
}
