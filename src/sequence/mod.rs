// This module describes a pulseq sequence, boiled down to the necessary info.
use std::rc::Rc;
use std::io::Write;

pub mod from_1_4;
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
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
}

pub struct Metadata {
    pub name: Option<String>,
    pub fov: Option<(f32, f32, f32)>,
    // Raster times are needed if time shapes are used.
    // These times are required by the 1.4+ parser, so if time shapes are used
    // but these values are None, it is a bug in the conversion process.
    pub grad_raster: Option<f32>,
    pub rf_raster: Option<f32>,
}

pub struct Block {
    /// Blocks are stored in a simple vector, isntead of a HashMap with their ID
    /// as value, because they are not referenced but executed top to bottom.
    /// Its own ID is stored inside of the Block for error reporting.
    pub id: u32,
    pub duration: f32,
    pub rf: Option<Rc<Rf>>,
    pub gx: Option<Rc<Gradient>>,
    pub gy: Option<Rc<Gradient>>,
    pub gz: Option<Rc<Gradient>>,
    pub adc: Option<Rc<Adc>>,
}

pub struct Rf {
    /// Unit: `[Hz]`
    pub amp: f32,
    /// Unit: `[rad]`
    pub phase: f32,
    pub amp_shape: Rc<Shape>,
    pub phase_shape: Rc<Shape>,
    /// Unit: `[s]`
    pub delay: f32,
    /// Unit: `[Hz]`
    pub freq: f32,
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
    pub num: u32,
    /// Unit: `[s]`
    pub dwell: f32,
    /// Unit: `[s]`
    pub delay: f32,
    /// Unit: `[Hz]`
    pub freq: f32,
    /// Unit: `[rad]`
    pub phase: f32,
}

pub struct Shape(pub Vec<f32>);
