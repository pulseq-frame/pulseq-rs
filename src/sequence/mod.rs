// This module describes a pulseq sequence, boiled down to the necessary info.
use std::{collections::HashMap, sync::Arc};

mod display;
pub mod from_1_4;

pub struct Sequence {
    pub time_raster: TimeRaster,
    pub name: Option<String>,
    pub fov: Option<(f32, f32, f32)>,
    pub definitions: HashMap<String, String>,
    pub blocks: Vec<Block>,
}

/// Before pulseq 1.4, definitions were not enforced. But despite this, the
/// RF and gradient shapes rely on a time raster! We solve this by always
/// providing the following definitions, filling them with the default
/// values of the Siemens interpreter if not provided in pre 1.4 sequences.
pub struct TimeRaster {
    pub grad: f32,
    pub rc: f32,
    pub adc: f32,
    pub block: f32,
}

impl Default for TimeRaster {
    fn default() -> Self {
        Self {
            grad: 10e-6,
            rc: 1e-6,
            adc: 0.1e-6,
            block: 10e-6,
        }
    }
}

pub struct Block {
    /// Blocks are stored in a simple vector, instead of a HashMap with their ID
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
    /// Unit: `[s]`
    pub delay: f32,
    /// Unit: `[Hz]`
    pub freq: f32,
    // Shapes
    pub amp_shape: Arc<Shape>,
    pub phase_shape: Arc<Shape>,
    pub time_shape: Option<Arc<Shape>>,
}

pub enum Gradient {
    Free {
        /// Unit: `[Hz/m]`
        amp: f32,
        /// Unit: `[s]`
        delay: f32,
        // Shapes
        shape: Arc<Shape>,
        time: Option<Arc<Shape>>,
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

// Helper functions

impl Rf {
    pub fn duration(&self, rf_raster: f32) -> f32 {
        self.delay + calc_shape_dur(&self.amp_shape, self.time_shape.as_deref(), rf_raster)
    }
}

impl Gradient {
    pub fn duration(&self, grad_raster: f32) -> f32 {
        match self {
            // TODO: duration calculation should take time_shape into account
            Gradient::Free {
                shape, delay, time, ..
            } => delay + calc_shape_dur(shape, time.as_deref(), grad_raster),
            Gradient::Trap {
                rise,
                flat,
                fall,
                delay,
                ..
            } => delay + rise + flat + fall,
        }
    }
}

fn calc_shape_dur(shape: &Shape, time: Option<&Shape>, raster: f32) -> f32 {
    if let Some(time) = time {
        time.0.last().cloned().unwrap_or(0.0) * raster
    } else {
        shape.0.len() as f32 * raster
    }
}
