use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

use super::*;
use crate::{
    error::{ConversionError, MissingDefinition, ParseFovError, SectionType},
    parse_file::{BlockDuration, Section, Version},
};

macro_rules! extract {
    ($sections:expr, $variant:ident) => {{
        let mut extracted = Vec::new();
        while let Some(index) = $sections
            .iter()
            .position(|x| matches!(x, Section::$variant(_)))
        {
            match $sections.swap_remove(index) {
                Section::$variant(section_data) => extracted.push(section_data),
                _ => unreachable!(),
            }
        }
        extracted
    }};
}

fn convert_sec<Data, Key: Eq + Hash, Val, F: FnMut(Data) -> Result<(Key, Val), ConversionError>>(
    ty: SectionType,
    sec_data: Vec<Vec<Data>>,
    f: F,
) -> Result<HashMap<Key, Val>, ConversionError> {
    let tmp = sec_data
        .into_iter()
        .flatten()
        .map(f)
        .collect::<Result<Vec<_>, ConversionError>>()?;
    let count = tmp.len();
    let tmp: HashMap<_, _> = tmp.into_iter().collect();

    if tmp.len() < count {
        Err(ConversionError::EventIdReuse(ty))
    } else {
        Ok(tmp)
    }
}

pub fn from_raw(mut sections: Vec<Section>) -> Result<Sequence, ConversionError> {
    // Destructure into single section or return error
    let [version]: [Version; 1] = extract!(sections, Version)
        .try_into()
        .map_err(|v: Vec<Version>| ConversionError::VersionSectionCount(v.len()))?;

    let Defs {
        name,
        fov,
        defs,
        time_raster,
    } = convert_defs(
        &version,
        extract!(sections, Definitions)
            .into_iter()
            .flatten()
            .collect(),
    )?;

    let mut shape_lib = ShapeLib::new(convert_sec(
        SectionType::Shapes,
        extract!(sections, Shapes),
        |shape| Ok((shape.id, Arc::new(Shape(shape.samples)))),
    )?)?;
    let delays = convert_sec(SectionType::Delays, extract!(sections, Delays), |delay| {
        Ok((delay.id, delay.delay))
    })?;
    let adcs = convert_sec(SectionType::Adcs, extract!(sections, Adcs), |adc| {
        Ok((
            adc.id,
            Arc::new(Adc {
                num: adc.num,
                dwell: adc.dwell,
                delay: adc.delay,
                freq: adc.freq,
                phase: adc.phase,
            }),
        ))
    })?;
    let rfs = convert_sec(SectionType::Rfs, extract!(sections, Rfs), |rf| {
        Ok((
            rf.id,
            Arc::new(Rf {
                amp: rf.amp,
                phase: rf.phase,
                amp_shape: shape_lib.get(rf.mag_id, rf.time_id)?,
                phase_shape: shape_lib.get(rf.phase_id, rf.time_id)?,
                // amp_shape: shapes[&rf.mag_id].clone(),
                // phase_shape: shapes[&rf.phase_id].clone(),
                // time_shape: (rf.time_id != 0).then(|| shapes[&rf.time_id].clone()),
                delay: rf.delay,
                freq: rf.freq,
            }),
        ))
    })?;
    let mut gradients = convert_sec(
        SectionType::Gradients,
        extract!(sections, Gradients),
        |grad| {
            Ok((
                grad.id,
                Arc::new(Gradient::Free {
                    amp: grad.amp,
                    shape: shape_lib.get(grad.shape_id, grad.time_id)?,
                    // shape: shapes[&grad.shape_id].clone(),
                    // time: if grad.time_id == 0 {
                    //     None
                    // } else {
                    //     Some(shapes[&grad.time_id].clone())
                    // },
                    delay: grad.delay,
                }),
            ))
        },
    )?;
    let traps = convert_sec(SectionType::Traps, extract!(sections, Traps), |trap| {
        Ok((
            trap.id,
            Arc::new(Gradient::Trap {
                amp: trap.amp,
                rise: trap.rise,
                flat: trap.flat,
                fall: trap.fall,
                delay: trap.delay,
            }),
        ))
    })?;

    // Gradients and Traps share keys
    let count = gradients.len() + traps.len();
    gradients.extend(traps);
    if gradients.len() < count {
        return Err(ConversionError::GradTrapIdReuse);
    }

    let mut blocks = extract!(sections, Blocks)
        .into_iter()
        .flatten()
        .map(|block| convert_block(block, &rfs, &gradients, &adcs, &delays, &time_raster))
        .collect::<Result<Vec<Block>, ConversionError>>()?;

    Ok(Sequence {
        name,
        fov,
        definitions: defs,
        time_raster,
        blocks,
    })
}

/// Simple helper struct to parse definitions into - might be removed after some
/// more refactoring, but as it's contained in this file this is not urgent.
struct Defs {
    name: Option<String>,
    fov: Option<(f32, f32, f32)>,
    time_raster: TimeRaster,
    defs: HashMap<String, String>,
}

fn convert_defs(version: &Version, defs: Vec<(String, String)>) -> Result<Defs, ConversionError> {
    let def_count = defs.len();
    let mut defs: HashMap<_, _> = defs.into_iter().collect();
    if defs.len() < def_count {
        // Duplicated key
        return Err(ConversionError::NonUniqueDefinition);
    }

    // Before 1.4, there is no spec on what's inside of a definition, so we
    // just directly return. Raster times are not exported by older exporters,
    // so we don't need to waste time trying to parse them.
    if !matches!(
        version,
        Version {
            major: 1,
            minor: 4,
            ..
        }
    ) {
        return Ok(Defs {
            name: None,
            fov: None,
            time_raster: TimeRaster::default(),
            defs,
        });
    }

    let time_raster = TimeRaster {
        grad: defs
            .remove("GradientRasterTime")
            .ok_or(MissingDefinition::GradientRasterTime)?
            .parse()?,
        rf: defs
            .remove("RadiofrequencyRasterTime")
            .ok_or(MissingDefinition::RadiofrequencyRasterTime)?
            .parse()?,
        adc: defs
            .remove("AdcRasterTime")
            .ok_or(MissingDefinition::AdcRasterTime)?
            .parse()?,
        block: defs
            .remove("BlockDurationRaster")
            .ok_or(MissingDefinition::BlockDurationRaster)?
            .parse()?,
    };
    let name = defs.remove("Name");
    let fov = defs.remove("FOV").map(parse_fov).transpose()?;

    Ok(Defs {
        name,
        fov,
        time_raster,
        defs,
    })
}

fn convert_block(
    block: crate::parse_file::Block,
    rfs: &HashMap<u32, Arc<Rf>>,
    gradients: &HashMap<u32, Arc<Gradient>>,
    adcs: &HashMap<u32, Arc<Adc>>,
    delays: &HashMap<u32, f32>,
    time_raster: &TimeRaster,
) -> Result<Block, ConversionError> {
    let err = |ty, id| ConversionError::BrokenRef { ty, id };
    use EventType::*;

    let rf = (block.rf != 0)
        .then(|| rfs.get(&block.rf).cloned().ok_or(err(Rf, block.rf)))
        .transpose()?;
    let gx = (block.gx != 0)
        .then(|| gradients.get(&block.gx).cloned().ok_or(err(Gx, block.gx)))
        .transpose()?;
    let gy = (block.gy != 0)
        .then(|| gradients.get(&block.gy).cloned().ok_or(err(Gy, block.gy)))
        .transpose()?;
    let gz = (block.gz != 0)
        .then(|| gradients.get(&block.gz).cloned().ok_or(err(Gz, block.gz)))
        .transpose()?;
    let adc = (block.adc != 0)
        .then(|| adcs.get(&block.adc).cloned().ok_or(err(Adc, block.adc)))
        .transpose()?;

    let duration = match block.dur {
        BlockDuration::Duration(dur) => dur as f32 * time_raster.block,
        BlockDuration::DelayId(delay) => {
            let delay = (delay != 0)
                .then(|| delays.get(&delay).cloned().ok_or(err(Delay, delay)))
                .transpose()?;

            [
                rf.as_ref().map(|rf| rf.duration(time_raster.rf)),
                gx.as_ref().map(|gx| gx.duration(time_raster.grad)),
                gy.as_ref().map(|gy| gy.duration(time_raster.grad)),
                gz.as_ref().map(|gz| gz.duration(time_raster.grad)),
                adc.as_ref().map(|adc| adc.duration()),
                delay,
            ]
            .into_iter()
            .flatten()
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
        }
    };

    Ok(Block {
        id: block.id,
        duration,
        rf,
        gx,
        gy,
        gz,
        adc,
    })
}

fn parse_fov(s: String) -> Result<(f32, f32, f32), ParseFovError> {
    let splits: Vec<_> = s.split_whitespace().collect();
    if splits.len() != 3 {
        Err(ParseFovError::WrongValueCount(splits.len()))
    } else {
        Ok((splits[0].parse()?, splits[1].parse()?, splits[2].parse()?))
    }
}

struct ShapeLib {
    shapes: HashMap<u32, Arc<Shape>>,
    memo: HashMap<(u32, u32), Arc<Shape>>,
}

impl ShapeLib {
    fn new(shapes: HashMap<u32, Arc<Shape>>) -> Result<Self, error::ConversionError> {
        // Checking this guarantee once makes later code easier
        if shapes.contains_key(&0) {
            Err(ConversionError::ShapeIndexZero)
        } else {
            Ok(Self {
                shapes,
                memo: HashMap::default(),
            })
        }
    }
    fn get(&mut self, shape_id: u32, time_id: u32) -> Result<Arc<Shape>, error::ConversionError> {
        let shape = self
            .shapes
            .get(&shape_id)
            .ok_or(ConversionError::ShapeNotFound(shape_id))?;

        if time_id == 0 {
            // Just a normal, continuous shape
            Ok(shape.clone())
        } else {
            // This shape skips some samples as defined by the time shape, expand it
            let time = self
                .shapes
                .get(&time_id)
                .ok_or(ConversionError::ShapeNotFound(time_id))?;

            // Avoid duplicates if shape was expanded before
            match self.memo.entry((shape_id, time_id)) {
                Entry::Occupied(e) => Ok(e.get().clone()),
                Entry::Vacant(e) => {
                    let expanded = Arc::new(expand_shape(shape, time)?);
                    Ok(e.insert(expanded).clone())
                }
            }
        }
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

    // Check if numbers in this shape are all integer, then convert to integers
    if time.0.iter().any(|x| x.fract() != 0.0) {
        return Err(ConversionError::TimeShapeNonInteger);
    }
    let time: Vec<_> = time.0.iter().map(|x| *x as u32).collect();

    // Do the actual conversion
    let mut expanded = Vec::with_capacity(*time.last().unwrap_or(&0) as usize);
    let mut amp = shape.0[0];

    for (len, &next_amp) in time.into_iter().zip(shape.0.iter()) {
        // If we are suddenly too long, time shape is not striclty increasing
        if expanded.len() > len as usize {
            return Err(ConversionError::TimeShapeNonIncreasing);
        }
        // Interpolate between amp and next_amp in line_len steps
        let line_len = len - expanded.len() as u32;
        for t in 0..line_len {
            let t = (t as f32 + 0.5) / line_len as f32;
            expanded.push(amp + t * (next_amp - amp));
        }
        amp = next_amp;
    }

    Ok(Shape(expanded))
}
