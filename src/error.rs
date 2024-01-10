use thiserror::Error;

use crate::parse_file::Version;

// TODO: Errors must be improved - by a LOT

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("GENERIC ERROR - IMPLEMENT MORE DETAILED VARIANTS")]
    Generic,
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    EzpcError(#[from] ezpc::EzpcError),
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Found unsupported version: {}.{}", .0.major, .0.minor)]
    UnsupportedVersion(Version),
}
