use std::collections::HashMap;

use crate::error::{ParseError, self};

mod common;
mod helpers;
mod pulseq_1_3;
mod pulseq_1_4;

// Pulseq is parsed into the following structs, which are modelled after the
// newest supported pulseq version. Older versions need to convert the data.
// This way, other code doesn't need to deal with version differences.

// Pulseq file format changes:
// v1.2: * Earliest tagged version on git and that we support
// v1.3: * Introduced extensions - Blocks are extended by one additional ID
// v1.4: * Removed Delay events - second value in Block changed from a delay ID
//         to the duration of the Block (in multiples of the block_dur_raster)
//       * Rf and Gradient now have an optional time_id for time shapes
//       * Also added mandatory definitions. Spec defines FOV units to be meters
//       * Shapes can be compressed
// vPtx: * Rf extended by two shape IDs for mag and phase shim arrays
//         https://gitlab.cs.fau.de/mrzero/pypulseq_rfshim

pub fn parse_file(source: &str) -> Result<Vec<Section>, error::Error> {
    let version = (helpers::nl().opt() + common::version() + ezpc::none_of("").repeat(0..))
        .parse_all(source)
        .map_err(|_| ParseError::Generic)?;

    match version {
        Version {
            major: 1, minor: 3, ..
        } => Ok(pulseq_1_3::file()
            .parse_all(source)?),
        Version {
            major: 1, minor: 4, ..
        } => Ok(pulseq_1_4::file()
            .parse_all(source)?),
        _ => Err(error::Error::UnsupportedVersion(version)),
    }
}

#[derive(Debug)]
pub enum Section {
    Version(Version),
    Signature(Signature),
    Definitions(Definitions),
    Blocks(Vec<Block>),
    Rfs(Vec<Rf>),
    Gradients(Vec<Gradient>),
    Traps(Vec<Trap>),
    Adcs(Vec<Adc>),
    Delays(Vec<Delay>),
    Extensions(Extensions),
    Shapes(Vec<Shape>),
}

#[derive(Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub rev_suppl: Option<String>,
}

#[derive(Debug)]
pub struct Signature {
    pub typ: String,
    pub hash: String,
}

#[derive(Debug)]
pub struct Definitions {
    pub grad_raster: f32,
    pub rf_raster: f32,
    pub adc_raster: f32,
    pub block_dur_raster: f32,
    pub name: Option<String>,
    pub fov: Option<(f32, f32, f32)>,
    pub rest: HashMap<String, String>,
}

#[derive(Debug)]
pub enum BlockDuration {
    Duration(u32),
    DelayId(u32),
}

#[derive(Debug)]
pub struct Block {
    pub id: u32,
    pub dur: BlockDuration,
    pub rf: u32,
    pub gx: u32,
    pub gy: u32,
    pub gz: u32,
    pub adc: u32,
    pub ext: u32,
}

#[derive(Debug)]
pub struct Rf {
    pub id: u32,
    /// `Hz`
    pub amp: f32,
    pub mag_id: u32,
    pub phase_id: u32,
    pub time_id: u32,
    /// `s` (from pulseq: `us`)
    pub delay: f32,
    /// `Hz`
    pub freq: f32,
    /// `rad`
    pub phase: f32,
}

#[derive(Debug)]
pub struct Gradient {
    pub id: u32,
    /// `Hz/m`
    pub amp: f32,
    pub shape_id: u32,
    pub time_id: u32,
    /// `s` (from pulseq: `us`)
    pub delay: f32,
}

#[derive(Debug)]
pub struct Trap {
    pub id: u32,
    /// `Hz/m`
    pub amp: f32,
    /// `s` (from pulseq: `us`)
    pub rise: f32,
    /// `s` (from pulseq: `us`)
    pub flat: f32,
    /// `s` (from pulseq: `us`)
    pub fall: f32,
    /// `s` (from pulseq: `us`)
    pub delay: f32,
}

#[derive(Debug)]
pub struct Adc {
    pub id: u32,
    pub num: u32,
    /// `s` (from pulseq: `ns`)
    pub dwell: f32,
    /// `s` (from pulseq: `us`)
    pub delay: f32,
    /// `Hz`
    pub freq: f32,
    /// `rad`
    pub phase: f32,
}

#[derive(Debug)]
pub struct Delay {
    pub id: u32,
    /// `s` (from pulseq: `us`)
    pub delay: f32,
}

#[derive(Debug)]
pub struct Extensions {
    pub refs: Vec<ExtensionRef>,
    pub specs: Vec<ExtensionSpec>,
}

#[derive(Debug)]
pub struct ExtensionRef {
    pub id: u32,
    pub spec_id: u32,
    pub obj_id: u32,
    pub next: u32,
}

#[derive(Debug)]
pub struct ExtensionSpec {
    pub id: u32,
    pub name: String,
    pub instances: Vec<ExtensionObject>,
}

#[derive(Debug)]
pub struct ExtensionObject {
    pub id: u32,
    pub data: String,
}

#[derive(Debug)]
pub struct Shape {
    pub id: u32,
    pub samples: Vec<f32>,
}
