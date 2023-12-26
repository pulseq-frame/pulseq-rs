// This module describes a pulseq sequence, boiled down to the necessary info.
use std::rc::Rc;
use std::io::Write;

mod from_1_4;
mod display;

#[test]
fn epi_se_rs() {
    let source = std::fs::read_to_string("assets/1.4.0/epi_se_rs.seq").unwrap();
    let sections = crate::parsers::pulseq_1_4::file()
        .parse_all(&source)
        .unwrap();
    let seq = Sequence::from_1_4(sections);

    let mut out = std::fs::File::create("assets/epi_se_rs.seq.dump").unwrap();
    write!(out, "{seq}").unwrap();
}

pub struct Sequence {
    metadata: Metadata,
    blocks: Vec<Block>,
}

pub struct Metadata {
    name: Option<String>,
    fov: Option<(f32, f32, f32)>,
    // Raster times are needed if time shapes are used.
    // These times are required by the 1.4+ parser, so if time shapes are used
    // but these values are None, it is a bug in the conversion process.
    grad_raster: Option<f32>,
    rf_raster: Option<f32>,
}

pub struct Block {
    /// Blocks are stored in a simple vector, isntead of a HashMap with their ID
    /// as value, because they are not referenced but executed top to bottom.
    /// Its own ID is stored inside of the Block for error reporting.
    id: u32,
    duration: f32,
    rf: Option<Rc<Rf>>,
    gx: Option<Rc<Gradient>>,
    gy: Option<Rc<Gradient>>,
    gz: Option<Rc<Gradient>>,
    adc: Option<Rc<Adc>>,
}

pub struct Rf {
    /// Unit: `[Hz]`
    amp: f32,
    /// Unit: `[rad]`
    phase: f32,
    amp_shape: Rc<Shape>,
    phase_shape: Rc<Shape>,
    /// Unit: `[s]`
    delay: f32,
    /// Unit: `[Hz]`
    freq: f32,
}

pub enum Gradient {
    Free {
        /// Unit: `[Hz/m]`
        amp: f32,
        shape: Rc<Shape>,
        time: Option<Rc<Shape>>,
        /// Unit: `[s]`
        delay: f32,
    },
    Trap {
        /// Unit: `[Hz/m]`
        amp: f32,
        /// Unit: `[s]`
        rise: f32,
        /// Unit: `[s]`
        flat: f32,
        /// Unit: `[s]`
        fall: f32,
        /// Unit: `[s]`
        delay: f32,
    },
}

pub struct Adc {
    num: u32,
    /// Unit: `[s]`
    dwell: f32,
    /// Unit: `[s]`
    delay: f32,
    /// Unit: `[Hz]`
    freq: f32,
    /// Unit: `[rad]`
    phase: f32,
}

pub struct Shape(Vec<f32>);
