// This module describes a pulseq sequence, boiled down to the necessary info.
use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    error::{self, EventType, ValidationError},
    parse_file::{self, Section},
};

mod display;
pub mod from_raw;

pub struct Sequence {
    pub time_raster: TimeRaster,
    pub name: Option<String>,
    pub fov: Option<(f64, f64, f64)>,
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
        Self::from_parsed_file(parse_file::parse_file(source)?)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, error::Error> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    pub fn validate(&self) -> Result<(), error::ValidationError> {
        // NOTE: We could check if block IDs are in some order or at least not
        // duplicated, but as they are never really used, this might be too strict

        // Check if no event is longer than the duration of its block
        for block in &self.blocks {
            // Passes through dur if its Some(..) and more than block.duration
            let check = |dur: Option<f64>, ty: EventType| {
                dur.map_or(Ok(()), |dur| {
                    if dur > block.duration + f64::EPSILON {
                        Err(ValidationError::EventTooLong {
                            ty,
                            block_id: block.id,
                            dur,
                            block_dur: block.duration,
                        })
                    } else {
                        Ok(())
                    }
                })
            };
            let grad_raster = self.time_raster.grad;

            check(
                block.rf.as_ref().map(|rf| rf.duration(self.time_raster.rf)),
                EventType::Rf,
            )?;
            check(
                block.gx.as_ref().map(|gx| gx.duration(grad_raster)),
                EventType::Gx,
            )?;
            check(
                block.gy.as_ref().map(|gy| gy.duration(grad_raster)),
                EventType::Gy,
            )?;
            check(
                block.gz.as_ref().map(|gz| gz.duration(grad_raster)),
                EventType::Gz,
            )?;
            check(block.adc.as_ref().map(|adc| adc.duration()), EventType::Adc)?;
        }

        // Check things like identical shape size and no negative times
        for block in &self.blocks {
            let id = block.id;
            use EventType::*;
            block.rf.as_ref().map_or(Ok(()), |x| x.validate(id))?;
            block.gx.as_ref().map_or(Ok(()), |x| x.validate(Gx, id))?;
            block.gy.as_ref().map_or(Ok(()), |x| x.validate(Gy, id))?;
            block.gz.as_ref().map_or(Ok(()), |x| x.validate(Gz, id))?;
            block.adc.as_ref().map_or(Ok(()), |x| x.validate(id))?;
        }

        Ok(())
    }
}

/// Before pulseq 1.4, definitions were not enforced. But despite this, the
/// RF and gradient shapes rely on a time raster! We solve this by always
/// providing the following definitions, filling them with the default
/// values of the Siemens interpreter if not provided in pre 1.4 sequences.
pub struct TimeRaster {
    pub grad: f64,
    pub rf: f64,
    pub adc: f64,
    pub block: f64,
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
    pub duration: f64,
    pub rf: Option<Arc<Rf>>,
    pub gx: Option<Arc<Gradient>>,
    pub gy: Option<Arc<Gradient>>,
    pub gz: Option<Arc<Gradient>>,
    pub adc: Option<Arc<Adc>>,
}

pub struct Rf {
    /// Unit: `[Hz]`
    pub amp: f64,
    /// Unit: `[rad]`
    pub phase: f64,
    /// Unit: `[s]`
    pub delay: f64,
    /// Unit: `[Hz]`
    pub freq: f64,
    // Shapes
    pub amp_shape: Arc<Shape>,
    pub phase_shape: Arc<Shape>,
    // pTx extension
    pub shim_shape: Option<(Arc<Shape>, Arc<Shape>)>,
}

pub enum Gradient {
    Free {
        /// Unit: `[Hz/m]`
        amp: f64,
        /// Unit: `[s]`
        delay: f64,
        // Shapes
        shape: Arc<Shape>,
    },
    Trap {
        /// Unit: `[Hz/m]`
        amp: f64,
        /// Unit: `[s]`
        rise: f64,
        /// Unit: `[s]`
        flat: f64,
        /// Unit: `[s]`
        fall: f64,
        /// Unit: `[s]`
        delay: f64,
    },
}

pub struct Adc {
    pub num: u32,
    /// Unit: `[s]`
    pub dwell: f64,
    /// Unit: `[s]`
    pub delay: f64,
    /// Unit: `[Hz]`
    pub freq: f64,
    /// Unit: `[rad]`
    pub phase: f64,
}

pub struct Shape(pub Vec<f64>);

// Helper functions and other impls

impl Rf {
    pub fn duration(&self, rf_raster: f64) -> f64 {
        self.delay + self.amp_shape.0.len() as f64 * rf_raster
    }

    fn validate(&self, block_id: u32) -> Result<(), error::ValidationError> {
        if self.phase_shape.0.len() != self.amp_shape.0.len() {
            Err(ValidationError::ShapeMismatch {
                ty: EventType::Rf,
                block_id,
                length_1: self.phase_shape.0.len(),
                length_2: self.amp_shape.0.len(),
            })?;
        }
        Ok(())
    }
}

impl Gradient {
    pub fn duration(&self, grad_raster: f64) -> f64 {
        match self {
            Gradient::Free { shape, delay, .. } => delay + shape.0.len() as f64 * grad_raster,
            Gradient::Trap {
                rise,
                flat,
                fall,
                delay,
                ..
            } => delay + rise + flat + fall,
        }
    }

    pub fn delay(&self) -> f64 {
        match self {
            Gradient::Free { delay, .. } => *delay,
            Gradient::Trap { delay, .. } => *delay,
        }
    }

    fn validate(&self, ty: EventType, block_id: u32) -> Result<(), error::ValidationError> {
        match self {
            Gradient::Free { delay, .. } => {
                if *delay < 0.0 {
                    Err(ValidationError::NegativeTiming {
                        ty,
                        block_id,
                        timing: *delay,
                    })
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
                    Err(ValidationError::NegativeTiming {
                        ty,
                        block_id,
                        timing: *rise,
                    })
                } else if *flat < 0.0 {
                    Err(ValidationError::NegativeTiming {
                        ty,
                        block_id,
                        timing: *flat,
                    })
                } else if *fall < 0.0 {
                    Err(ValidationError::NegativeTiming {
                        ty,
                        block_id,
                        timing: *fall,
                    })
                } else if *delay < 0.0 {
                    Err(ValidationError::NegativeTiming {
                        ty,
                        block_id,
                        timing: *delay,
                    })
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Adc {
    pub fn duration(&self) -> f64 {
        self.delay + self.num as f64 * self.dwell
    }

    fn validate(&self, block_id: u32) -> Result<(), error::ValidationError> {
        if self.dwell < 0.0 {
            Err(ValidationError::NegativeTiming {
                ty: EventType::Adc,
                block_id,
                timing: self.dwell,
            })
        } else if self.delay < 0.0 {
            Err(ValidationError::NegativeTiming {
                ty: EventType::Adc,
                block_id,
                timing: self.delay,
            })
        } else {
            Ok(())
        }
    }
}
