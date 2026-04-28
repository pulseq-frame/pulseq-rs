mod error;
mod parse_file;
mod sequence;

pub use error::Error;
pub use parse_file::parse_file;
pub use sequence::{Adc, Block, Extension, Gradient, Rf, Sequence, Shape, TimeRaster};

/// Raw parser types, mirroring the .seq file structure directly.
/// Use for tooling that needs to see IDs, references, and shapes as written.
pub mod raw {
    pub use crate::parse_file::{
        Adc, Block, BlockDuration, Delay, ExtensionObject, ExtensionRef, ExtensionSpec, Extensions,
        Gradient, Rf, Section, Shape, Signature, Trap, Version,
    };
}
