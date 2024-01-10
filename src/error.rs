use ezpc::EzpcError;
use thiserror::Error;

use crate::parse_file::Version;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("GENERIC ERROR - IMPLEMENT MORE DETAILED VARIANTS")]
    Generic,
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Parse error")]
    EzpcError(String),
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Found unsupported version: {}.{}", .0.major, .0.minor)]
    UnsupportedVersion(Version),
}

/// TODO: There is a problem with lifetimes, maybe Ezpc should be changed so that
/// the error type does not contain a reference to the source anymore...
impl<'a> From<EzpcError<'a>> for Error {
    fn from(value: EzpcError<'a>) -> Self {
        Error::EzpcError(value.to_string())
    }
}
