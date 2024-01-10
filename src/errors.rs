use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("GENERIC ERROR - IMPLEMENT MORE DETAILED VARIANTS")]
    Generic,
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}
