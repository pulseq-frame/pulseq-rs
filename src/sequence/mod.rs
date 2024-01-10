// This module describes a pulseq sequence, boiled down to the necessary info.
use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    error,
    parse_file::{self, Section},
};

mod display;
pub mod from_raw;

pub struct Sequence {
    pub time_raster: TimeRaster,
    pub name: Option<String>,
    pub fov: Option<(f32, f32, f32)>,
    pub definitions: HashMap<String, String>,
    pub blocks: Vec<Block>,
}

impl Sequence {
    pub fn from_parsed_file(sections: Vec<Section>) -> Result<Self, error::Error> {
        let tmp = from_raw::from_raw(sections)?;
        tmp.validate()?;
        Ok(tmp)
    }

    pub fn from_source(source: &str) -> Result<Self, error::Error> {
        parse_file::parse_file(source).and_then(Self::from_parsed_file)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, error::Error> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    pub fn validate(&self) -> Result<(), error::Error> {
        // NOTE: We could check if block IDs are in some order or at least not
        // duplicated, but as they are never really used, this might be too strict

        // Check if no event is longer than the duration of its block
        for block in &self.blocks {
            let check = |dur: Option<f32>, err| {
                dur.map_or(Ok(()), |d| {
                    if d > block.duration + f32::EPSILON {
                        Err(err)
                    } else {
                        Ok(())
                    }
                })
            };
            let grad_raster = self.time_raster.grad;

            check(
                block.rf.as_ref().map(|rf| rf.duration(self.time_raster.rf)),
                error::Error::ParseError(error::ParseError::Generic),
            )?;
            check(
                block.gx.as_ref().map(|gx| gx.duration(grad_raster)),
                error::Error::ParseError(error::ParseError::Generic),
            )?;
            check(
                block.gy.as_ref().map(|gy| gy.duration(grad_raster)),
                error::Error::ParseError(error::ParseError::Generic),
            )?;
            check(
                block.gz.as_ref().map(|gz| gz.duration(grad_raster)),
                error::Error::ParseError(error::ParseError::Generic),
            )?;
            check(
                block.adc.as_ref().map(|adc| adc.duration()),
                error::Error::ParseError(error::ParseError::Generic),
            )?;
        }

        // Check things like identical shape size and no negative times
        for block in &self.blocks {
            block.rf.as_ref().map_or(Ok(()), |tmp| tmp.validate())?;
            block.gx.as_ref().map_or(Ok(()), |tmp| tmp.validate())?;
            block.gy.as_ref().map_or(Ok(()), |tmp| tmp.validate())?;
            block.gz.as_ref().map_or(Ok(()), |tmp| tmp.validate())?;
            block.adc.as_ref().map_or(Ok(()), |tmp| tmp.validate())?;
        }

        Ok(())
    }
}

/// Before pulseq 1.4, definitions were not enforced. But despite this, the
/// RF and gradient shapes rely on a time raster! We solve this by always
/// providing the following definitions, filling them with the default
/// values of the Siemens interpreter if not provided in pre 1.4 sequences.
pub struct TimeRaster {
    pub grad: f32,
    pub rf: f32,
    pub adc: f32,
    pub block: f32,
}

impl Default for TimeRaster {
    fn default() -> Self {
        Self {
            grad: 10e-6,
            rf: 1e-6,
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

// Helper functions and other impls

impl Rf {
    pub fn duration(&self, rf_raster: f32) -> f32 {
        self.delay + calc_shape_dur(&self.amp_shape, self.time_shape.as_deref(), rf_raster)
    }

    fn validate(&self) -> Result<(), error::Error> {
        if self.phase_shape.0.len() != self.amp_shape.0.len() {
            return Err(error::Error::ParseError(error::ParseError::Generic));
        }
        if let Some(time_shape) = &self.time_shape {
            if time_shape.0.len() != self.amp_shape.0.len() {
                return Err(error::Error::ParseError(error::ParseError::Generic));
            }
        }
        Ok(())
    }
}

impl Gradient {
    pub fn duration(&self, grad_raster: f32) -> f32 {
        match self {
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

    fn validate(&self) -> Result<(), error::Error> {
        match self {
            Gradient::Free {
                delay, shape, time, ..
            } => {
                if *delay < 0.0 {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else if time.as_ref().map_or(false, |t| shape.0.len() != t.0.len()) {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else {
                    Ok(())
                }
            }
            Gradient::Trap {
                rise,
                flat,
                fall,
                delay,
                ..
            } => {
                if *rise < 0.0 {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else if *flat < 0.0 {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else if *fall < 0.0 {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else if *delay < 0.0 {
                    Err(error::Error::ParseError(error::ParseError::Generic))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Adc {
    pub fn duration(&self) -> f32 {
        self.delay + self.num as f32 * self.dwell
    }
    fn validate(&self) -> Result<(), error::Error> {
        if self.dwell < 0.0 {
            Err(error::Error::ParseError(error::ParseError::Generic))
        } else if self.delay < 0.0 {
            Err(error::Error::ParseError(error::ParseError::Generic))
        } else {
            Ok(())
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
