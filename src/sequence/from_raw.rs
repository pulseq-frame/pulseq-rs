use std::{collections::HashMap, hash::Hash};

use num_complex::Complex64;

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
        required_exts,
        time_raster,
    } = convert_defs(
        &version,
        extract!(sections, Definitions)
            .into_iter()
            .flatten()
            .collect(),
    )?;

    for ext in required_exts {
        match ext.as_str() {
            "label" | "labelset" | "labelinc" | "triggers" | "delays" | "rotations"
            | "rf_shims" => (),
            _ => panic!("unsupported required extension: '{ext}'"),
        }
    }

    let mut shape_lib = ShapeLib::new(convert_sec(
        SectionType::Shapes,
        extract!(sections, Shapes),
        |shape| Ok((shape.id, Arc::new(Shape(shape.samples)))),
    )?)?;
    let delays = convert_sec(SectionType::Delays, extract!(sections, Delays), |delay| {
        Ok((delay.id, delay.delay))
    })?;
    let adcs = convert_sec(SectionType::Adcs, extract!(sections, Adcs), |adc| {
        let phase_shape = if adc.phase_shape_id == 0 {
            None
        } else {
            Some(shape_lib.get(adc.phase_shape_id, 0)?)
        };
        Ok((
            adc.id,
            Arc::new(Adc {
                num: adc.num,
                dwell: adc.dwell,
                delay: adc.delay,
                freq: (adc.freq_rel, adc.freq_off),
                phase: (adc.phase_rel, adc.phase_off),
                phase_shape,
            }),
        ))
    })?;
    let rfs = convert_sec(SectionType::Rfs, extract!(sections, Rfs), |rf| {
        let shape = shape_lib.get_complex(rf.mag_id, rf.phase_id, rf.time_id)?;
        let shim_shape = match rf.shim_id {
            Some((mag_id, phase_id)) => Some(shape_lib.get_complex(mag_id, phase_id, 0)?),
            None => None,
        };
        Ok((
            rf.id,
            Arc::new(Rf {
                amp: rf.amp,
                phase: (rf.phase_rel, rf.phase_off),
                shape,
                delay: rf.delay,
                // TODO: calc from shape if not set
                center: rf.center.unwrap_or(-1.0),
                freq: (rf.freq_rel, rf.freq_off),
                shim_shape,
                rf_use: RfUse::from_char(rf.rf_use).expect("parser accepted invalid char"),
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

    let exts: HashMap<u32, Vec<Extension>> = extract!(sections, Extensions)
        .into_iter()
        .flat_map(convert_exts)
        .collect();

    let blocks = extract!(sections, Blocks)
        .into_iter()
        .flatten()
        .map(|block| convert_block(block, &rfs, &gradients, &adcs, &delays, &time_raster, &exts))
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
    fov: Option<(f64, f64, f64)>,
    time_raster: TimeRaster,
    /// lower-cased strings from "RequiredExtensions" definition
    required_exts: Vec<String>,
    defs: HashMap<String, String>,
}

fn convert_defs(version: &Version, defs: Vec<(String, String)>) -> Result<Defs, ConversionError> {
    let def_count = defs.len();
    let mut defs: HashMap<_, _> = defs.into_iter().collect();
    if defs.len() < def_count {
        // Duplicated key
        return Err(ConversionError::NonUniqueDefinition);
    }

    // Supported since pulseq 1.5 but earlier versions should not accidentally export this
    let required_exts: Vec<String> = defs
        .remove("RequiredExtensions")
        .unwrap_or(String::new())
        .split_whitespace()
        .map(|s| s.trim().to_lowercase())
        .collect();

    // Before 1.4, there is no spec on what's inside of a definition, so we
    // just directly return. Raster times are not exported by older exporters,
    // so we don't need to waste time trying to parse them.
    if version.major == 1 && version.minor < 4 {
        return Ok(Defs {
            name: None,
            fov: None,
            time_raster: TimeRaster::default(),
            required_exts,
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
        required_exts,
        defs,
    })
}

/// Very rough impl just to get something going- values are (ext_name, obj_data)
fn convert_exts(exts: crate::parse_file::Extensions) -> HashMap<u32, Vec<Extension>> {
    // Indexed by (spec_id, obj_id), contains (spec_name, spec_data)
    let specs: HashMap<(u32, u32), Extension> = exts
        .specs
        .iter()
        .flat_map(|spec| {
            spec.instances
                .iter()
                .map(|obj| ((spec.id, obj.id), Extension::parse(&spec.name, &obj.data)))
        })
        .collect();

    let refs: HashMap<u32, crate::parse_file::ExtensionRef> =
        exts.refs.iter().map(|ext| (ext.id, *ext)).collect();

    fn walk_linked_ref_list(
        refs: &HashMap<u32, parse_file::ExtensionRef>,
        mut ext_id: u32,
        specs: &HashMap<(u32, u32), Extension>,
    ) -> Vec<Extension> {
        let mut tmp = Vec::new();
        // max depth is 50 - hardcoded, maybe should add cycle detector or proper error return val
        for _ in 0..50 {
            let ext_ref = &refs[&ext_id];
            tmp.push(specs[&(ext_ref.spec_id, ext_ref.obj_id)].clone());

            ext_id = ext_ref.next;
            if ext_id == 0 {
                return tmp;
            }
        }
        panic!("Max extension linked list depth (50) reached")
    }

    let mut parsed = HashMap::new();
    for ext_ref in &exts.refs {
        parsed.insert(ext_ref.id, walk_linked_ref_list(&refs, ext_ref.id, &specs));
    }
    parsed
}

fn convert_block(
    block: crate::parse_file::Block,
    rfs: &HashMap<u32, Arc<Rf>>,
    gradients: &HashMap<u32, Arc<Gradient>>,
    adcs: &HashMap<u32, Arc<Adc>>,
    delays: &HashMap<u32, f64>,
    time_raster: &TimeRaster,
    exts: &HashMap<u32, Vec<Extension>>,
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
        BlockDuration::Duration(dur) => dur as f64 * time_raster.block,
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

    // TODO: add Ext event type in error (see code above) to return error instead of unwrapping
    let ext = (block.ext != 0)
        .then(|| exts.get(&block.ext).cloned().unwrap())
        .unwrap_or_default();

    Ok(Block {
        id: block.id,
        duration,
        rf,
        gx,
        gy,
        gz,
        adc,
        ext,
    })
}

fn parse_fov(s: String) -> Result<(f64, f64, f64), ParseFovError> {
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
    complex_memo: HashMap<(u32, u32, u32), Arc<ComplexShape>>,
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
                complex_memo: HashMap::default(),
            })
        }
    }
    fn get(&mut self, shape_id: u32, time_id: u32) -> Result<Arc<Shape>, ConversionError> {
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

    fn get_complex(
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
            let t = (t as f64 + 0.5) / line_len as f64;
            expanded.push(amp + t * (next_amp - amp));
        }
        amp = next_amp;
    }

    Ok(Shape(expanded))
}
