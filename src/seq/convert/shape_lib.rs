use std::{collections::HashMap, sync::Arc};

use num_complex::Complex64;

use crate::{error::ConversionError, seq::ComplexShape, seq::Shape};

pub struct ShapeLib {
    shapes: HashMap<u32, Arc<Shape>>,
    memo: HashMap<(u32, u32), Arc<Shape>>,
    complex_memo: HashMap<(u32, u32, u32), Arc<ComplexShape>>,
}

impl ShapeLib {
    pub fn new(shapes: HashMap<u32, Arc<Shape>>) -> Result<Self, ConversionError> {
        // Checking this guarantee once makes later code easier
        if shapes.contains_key(&0) {
            Err(ConversionError::ShapeIndexZero)
        } else {
            Ok(Self {
                shapes,
                memo: HashMap::default(),
                complex_memo: HashMap::default(),
            })
        }
    }
    pub fn get(&mut self, shape_id: u32, time_id: u32) -> Result<Arc<Shape>, ConversionError> {
        let key = (shape_id, time_id);
        if let Some(cached) = self.memo.get(&key) {
            return Ok(cached.clone());
        }

        // First we get the shape itself,
        // then we see if can use it directly or need to expand it with the time shape
        let shape = self
            .shapes
            .get(&shape_id)
            .ok_or(ConversionError::ShapeNotFound(shape_id))?;

        let shape = if time_id == 0 {
            shape.clone()
        } else {
            let time = self
                .shapes
                .get(&time_id)
                .ok_or(ConversionError::ShapeNotFound(time_id))?;

            Arc::new(expand_shape(shape, time)?)
        };

        self.memo.insert(key, shape.clone());
        Ok(shape)
    }

    pub fn get_complex(
        &mut self,
        mag_id: u32,
        phase_id: u32,
        time_id: u32,
    ) -> Result<Arc<ComplexShape>, ConversionError> {
        let key = (mag_id, phase_id, time_id);
        if let Some(cached) = self.complex_memo.get(&key) {
            return Ok(cached.clone());
        }

        let mag = self.get(mag_id, time_id)?;
        let phase = self.get(phase_id, time_id)?;

        if mag.0.len() != phase.0.len() {
            // TODO: can't be the time shape (is the same) but only happen when time_id==0 and shapes mismatch
            return Err(ConversionError::TimeShapeMismatch {
                shape_len: mag.0.len(),
                time_len: phase.0.len(),
            });
        }

        let samples = mag
            .0
            .iter()
            .zip(phase.0.iter())
            .map(|(&a, &p)| Complex64::from_polar(a, p * std::f64::consts::TAU))
            .collect();

        let shape = Arc::new(ComplexShape(samples));
        self.complex_memo.insert(key, shape.clone());
        Ok(shape)
    }
}

/// Here we do interpolation as given by the time shape. The spec unfortunately does not
/// define at all how to use the time shapes, but here is what I found by looking at
/// how they are used in example scripts:
/// The shape is defined by a series of time points that are on the EDGES of the samples:
/// A trapezoid is defined by [0, rise, rise + flat, rise + flat + fall] while the first
/// sample should be at [0.5 * dwell, ...]. This means that a time shape [0, 100] is not
/// 101 units long but indeed 100 - with the samples being located at [0.5, 1.5, ..., 99.5].
/// This is how the scripts use the custom time shape feature and what would be
/// consistent with the make_..._pulse functions.
/// But this also means that the amplitudes are given in-between samples and need
/// to be interpolated, while shapes without time-shapes are definded on-sample.
/// This is probably an oversight of pulseq, but seems to be the best approach of
/// implementing time shape expansion right now.
/// In addition, we use linar interpolation (as it seems to be expected when using
/// this feature for trap grads). The spec does not say anything about interpolation at all.
/// 
/// TODO: check if this actually correct and how interpreters expect it.
fn expand_shape(shape: &Arc<Shape>, time: &Arc<Shape>) -> Result<Shape, ConversionError> {
    if shape.0.len() != time.0.len() {
        return Err(ConversionError::TimeShapeMismatch {
            shape_len: shape.0.len(),
            time_len: time.0.len(),
        });
    }

    // Probably a bug but technically not an error
    if shape.0.is_empty() {
        return Ok(Shape(Vec::new()));
    }

    // Convert time shape to integers (after a check if that's okay)
    if time.0.iter().any(|x| x.fract() != 0.0) {
        return Err(ConversionError::TimeShapeNonInteger);
    }
    if time.0.iter().any(|x| *x < 0.0) {
        return Err(ConversionError::TimeShapeNegative);
    }
    let time: Vec<_> = time.0.iter().map(|x| *x as u32).collect();

    // Do the actual conversion
    let mut expanded = Vec::with_capacity(*time.last().unwrap_or(&0) as usize);
    let mut amp = shape.0[0];

    for (len, &next_amp) in time.into_iter().zip(shape.0.iter()) {
        // If we are suddenly too long, time shape is not strictly increasing
        if expanded.len() > len as usize {
            return Err(ConversionError::TimeShapeNonIncreasing);
        }
        // Interpolate between amp and next_amp in line_len steps
        let line_len = len - expanded.len() as u32;
        for t in 0..line_len {
            let t = (t as f64 + 0.5) / line_len as f64;
            expanded.push(amp + t * (next_amp - amp));
        }
        amp = next_amp;
    }

    Ok(Shape(expanded))
}
