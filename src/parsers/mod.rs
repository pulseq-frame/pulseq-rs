use std::collections::HashMap;

pub mod common;
pub mod pulseq_1_4_0;

// Pulseq is parsed into the following structs, which are modelled after the
// newest supported pulseq version. Older versions need to convert the data.
// This way, other code doesn't need to deal with version differences.

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
    Extensions(Extensions),
    Shapes(Vec<Shape>),
}

#[derive(Debug)]
pub struct Version {
    major: u32,
    minor: u32,
    revision: u32,
    rev_suppl: Option<String>,
}

#[derive(Debug)]
pub struct Signature {
    typ: String,
    hash: String,
}

#[derive(Debug)]
pub struct Definitions {
    grad_raster: f32,
    rf_raster: f32,
    adc_raster: f32,
    block_dur_raster: f32,
    name: Option<String>,
    fov: Option<(f32, f32, f32)>,
    total_duration: Option<f32>,
    rest: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Block {
    id: u32,
    duration: u32,
    rf: u32,
    gx: u32,
    gy: u32,
    gz: u32,
    adc: u32,
    ext: u32,
}

#[derive(Debug)]
pub struct Rf {
    id: u32,
    /// `Hz`
    amp: f32,
    mag_id: u32,
    phase_id: u32,
    time_id: u32,
    /// `s` (from pulseq: `us`)
    delay: f32,
    /// `Hz`
    freq: f32,
    /// `rad`
    phase: f32,
}

#[derive(Debug)]
pub struct Gradient {
    id: u32,
    /// `Hz/m`
    amp: f32,
    shape_id: u32,
    time_id: u32,
    /// `s` (from pulseq: `us`)
    delay: f32,
}

#[derive(Debug)]
pub struct Trap {
    id: u32,
    /// `Hz/m`
    amp: f32,
    /// `s` (from pulseq: `us`)
    rise: f32,
    /// `s` (from pulseq: `us`)
    flat: f32,
    /// `s` (from pulseq: `us`)
    fall: f32,
    /// `s` (from pulseq: `us`)
    delay: f32,
}

#[derive(Debug)]
pub struct Adc {
    id: u32,
    num: u32,
    /// `s` (from pulseq: `ns`)
    dwell: f32,
    /// `s` (from pulseq: `us`)
    delay: f32,
    /// `Hz`
    freq: f32,
    /// `rad`
    phase: f32,
}

#[derive(Debug)]
pub struct Extensions {
    refs: Vec<ExtensionRef>,
    specs: Vec<ExtensionSpec>,
}

#[derive(Debug)]
pub struct ExtensionRef {
    id: u32,
    spec_id: u32,
    obj_id: u32,
    next: u32,
}

#[derive(Debug)]
pub struct ExtensionSpec {
    id: u32,
    name: String,
    instances: Vec<ExtensionObject>,
}

#[derive(Debug)]
pub struct ExtensionObject {
    id: u32,
    data: String,
}

#[derive(Debug)]
pub struct Shape {
    id: u32,
    samples: Vec<f32>,
}
