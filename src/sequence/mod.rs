// This module describes a pulseq sequence, boiled down to the necessary info.
use std::io::Write;
use std::sync::Arc;

mod display;
pub mod from_1_4;

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
    pub grad_raster: f32,
    pub rf_raster: f32,
    pub adc_raster: f32,
    pub block_raster: f32,
}

pub struct Block {
    /// Blocks are stored in a simple vector, isntead of a HashMap with their ID
    /// as value, because they are not referenced but executed top to bottom.
    /// Its own ID is stored inside of the Block for error reporting.
    pub id: u32,
    pub duration: f32,
    pub rf: Option<Arc<Rf>>,
    pub gx: Option<Arc<Gradient>>,
    pub gy: Option<Arc<Gradient>>,
    pub gz: Option<Arc<Gradient>>,
    pub adc: Option<Arc<Adc>>,
}

pub struct Rf {
    /// Unit: `[Hz]`
    pub amp: f32,
    /// Unit: `[rad]`
    pub phase: f32,
    pub amp_shape: Arc<Shape>,
    pub phase_shape: Arc<Shape>,
    /// Unit: `[s]`
    pub delay: f32,
    /// Unit: `[Hz]`
    pub freq: f32,
}

pub enum Gradient {
    Free {
        /// Unit: `[Hz/m]`
        amp: f32,
        shape: Arc<Shape>,
        time: Option<Arc<Shape>>,
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

impl Gradient {
    pub fn duration(&self, grad_raster: f32) -> f32 {
        match self {
            // TODO: duration calculation should take time_shape into account
            Gradient::Free {
                amp,
                shape,
                time,
                delay,
            } => delay + shape.0.len() as f32 * grad_raster,
            Gradient::Trap {
                amp,
                rise,
                flat,
                fall,
                delay,
            } => delay + rise + flat + fall,
        }
    }
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
