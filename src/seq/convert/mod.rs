use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::Sequence;
use crate::{
    error::{ConversionError, SectionType},
    raw, seq,
};

mod definitions;
mod sections;
mod shape_lib;

use definitions::Defs;
use sections::{SectionData, get_section_data};
use shape_lib::ShapeLib;

pub fn from_raw(mut sections: Vec<raw::Section>) -> Result<Sequence, ConversionError> {
    let [version]: [raw::Version; 1] = get_section_data(&mut sections)
        .try_into()
        .map_err(|v: Vec<raw::Version>| ConversionError::VersionSectionCount(v.len()))?;

    let defs = Defs::from_raw(&version, get_section_data(&mut sections))?;
    check_ext_support(&defs.required_exts)?;

    // Build the raw shape map first so the gradient-boundary pass can read
    // raw samples; clone (cheap — Arc refcount bumps) for ShapeLib to own.
    let raw_shapes: HashMap<u32, Arc<Vec<f64>>> =
        map_section_data(&mut sections, |shape: raw::Shape| {
            Ok((shape.id, Arc::new(shape.samples)))
        })?;
    let mut shapes = ShapeLib::new(raw_shapes.clone())?;

    let delays = map_section_data(&mut sections, |delay: raw::Delay| {
        Ok((delay.id, delay.delay))
    })?;

    let adcs = map_section_data(&mut sections, |adc: raw::Adc| {
        // SPEC NOTE: pulseq has no `time_id` for ADC phase shapes; the array
        // is sampled per ADC sample at `dwell`. We pass `time_id = 0` so the
        // shape gets a synthesised `time = (1..=n)` matching every other
        // uniform shape, letting downstream code treat shapes uniformly.
        let phase_shape = if adc.phase_shape_id == 0 {
            None
        } else {
            Some(shapes.get(adc.phase_shape_id, 0)?)
        };
        Ok((
            adc.id,
            Arc::new(seq::Adc {
                num: adc.num,
                dwell: adc.dwell,
                delay: adc.delay,
                freq: (adc.freq_rel, adc.freq_off),
                phase: (adc.phase_rel, adc.phase_off),
                phase_shape,
            }),
        ))
    })?;

    let rfs = map_section_data(&mut sections, |rf: raw::Rf| {
        let shape = shapes.get_complex(rf.mag_id, rf.phase_id, rf.time_id)?;
        let shim_shape = match rf.shim_id {
            Some((mag_id, phase_id)) => Some(shapes.get_complex(mag_id, phase_id, 0)?),
            None => None,
        };
        let center = rf
            .center
            .unwrap_or_else(|| rf.delay + defs.time_raster.rf * shape.calc_center() as f64);

        Ok((
            rf.id,
            Arc::new(seq::Rf {
                amp: rf.amp,
                phase: (rf.phase_rel, rf.phase_off),
                shape,
                delay: rf.delay,
                center,
                freq: (rf.freq_rel, rf.freq_off),
                shim_shape,
                rf_use: rf.rf_use,
            }),
        ))
    })?;

    // Extract raw gradients, traps, and blocks. We need raw access to all three
    // before converting any of them so the block-context pass can compute
    // pre-1.5 first/last per gradient id.
    let raw_gradients: Vec<raw::Gradient> = get_section_data(&mut sections);
    let raw_traps: Vec<raw::Trap> = get_section_data(&mut sections);
    let raw_blocks: Vec<raw::Block> = get_section_data(&mut sections);

    let raw_grad_map: HashMap<u32, &raw::Gradient> =
        raw_gradients.iter().map(|g| (g.id, g)).collect();
    if raw_grad_map.len() < raw_gradients.len() {
        return Err(ConversionError::EventIdReuse(SectionType::Gradients));
    }
    let raw_trap_map: HashMap<u32, &raw::Trap> = raw_traps.iter().map(|t| (t.id, t)).collect();
    if raw_trap_map.len() < raw_traps.len() {
        return Err(ConversionError::EventIdReuse(SectionType::Traps));
    }

    let grad_boundaries = compute_grad_boundaries(
        &raw_blocks,
        &raw_grad_map,
        &raw_trap_map,
        &raw_shapes,
        &defs.time_raster,
    )?;

    let mut gradients: HashMap<u32, Arc<seq::Gradient>> =
        HashMap::with_capacity(raw_gradients.len());
    for grad in &raw_gradients {
        let (first_abs, last_abs) = grad_boundaries
            .get(&grad.id)
            .copied()
            .unwrap_or((grad.first.unwrap_or(0.0), grad.last.unwrap_or(0.0)));
        let (first_norm, last_norm) = if grad.amp != 0.0 {
            (first_abs / grad.amp, last_abs / grad.amp)
        } else {
            (0.0, 0.0)
        };
        let shape =
            shapes.get_with_boundaries(grad.shape_id, grad.time_id, first_norm, last_norm)?;
        gradients.insert(
            grad.id,
            Arc::new(seq::Gradient::Free {
                amp: grad.amp,
                shape,
                delay: grad.delay,
            }),
        );
    }

    let traps: HashMap<u32, Arc<seq::Gradient>> = raw_traps
        .iter()
        .map(|trap| {
            (
                trap.id,
                Arc::new(seq::Gradient::Trap {
                    amp: trap.amp,
                    rise: trap.rise,
                    flat: trap.flat,
                    fall: trap.fall,
                    delay: trap.delay,
                }),
            )
        })
        .collect();

    // Gradients and Traps share keys
    let count = gradients.len() + traps.len();
    gradients.extend(traps);
    if gradients.len() < count {
        return Err(ConversionError::GradTrapIdReuse);
    }

    let ext_refs: Vec<raw::ExtensionRef> = get_section_data(&mut sections);
    let ext_specs: Vec<raw::ExtensionSpec> = get_section_data(&mut sections);
    let (exts, soft_delay_hints) = convert_exts(ext_refs, ext_specs)?;

    // We do not use map_section_data here since we do not care about block ids
    let blocks = raw_blocks
        .into_iter()
        .map(|block: raw::Block| {
            convert_block(
                block,
                &rfs,
                &gradients,
                &adcs,
                &delays,
                &defs.time_raster,
                &exts,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Sequence {
        name: defs.name,
        fov: defs.fov,
        definitions: defs.defs,
        time_raster: defs.time_raster,
        blocks,
        soft_delay_hints,
    })
}

fn check_ext_support(required: &[String]) -> Result<(), ConversionError> {
    for ext in required {
        match ext.as_str() {
            "label" | "labelset" | "labelinc" | "triggers" | "delays" | "rotations"
            | "rf_shims" => (),
            _ => return Err(ConversionError::UnsupportedExtension(ext.to_owned())),
        }
    }
    Ok(())
}

/// Wrapper around get_section_data that applies a mapping func and hashes by id.
fn map_section_data<T, Val, F>(
    sections: &mut Vec<raw::Section>,
    f: F,
) -> Result<HashMap<u32, Val>, ConversionError>
where
    T: SectionData,
    F: FnMut(T) -> Result<(u32, Val), ConversionError>,
{
    let data: Vec<T> = get_section_data(sections);
    let raw_count = data.len();
    let data = data
        .into_iter()
        .map(f)
        .collect::<Result<HashMap<u32, Val>, ConversionError>>()?;

    if data.len() < raw_count {
        Err(ConversionError::EventIdReuse(T::SECTION_TYPE))
    } else {
        Ok(data)
    }
}

fn convert_exts(
    ext_refs: Vec<raw::ExtensionRef>,
    ext_specs: Vec<raw::ExtensionSpec>,
) -> Result<(HashMap<u32, Vec<seq::Extension>>, HashMap<u32, String>), ConversionError> {
    // Walk every (spec, obj) pair, parsing the extension and - for `delays`
    // specs - capturing the hint into a single sequence-level table. Hints
    // are not stored on `Extension::Delay` itself (see seq/extensions.rs).
    let mut parsed_specs: HashMap<(u32, u32), seq::Extension> = HashMap::new();
    let mut soft_delay_hints: HashMap<u32, String> = HashMap::new();
    for spec in &ext_specs {
        for obj in &spec.instances {
            let ext = seq::Extension::parse(&spec.name, &obj.data)?;

            if let seq::Extension::Delay { id, .. } = &ext {
                // parse_delay validated 4 whitespace-separated fields, so
                // parts[3] (the hint) is guaranteed to exist.
                let hint = obj
                    .data
                    .split_whitespace()
                    .nth(3)
                    .unwrap_or_default()
                    .to_owned();
                match soft_delay_hints.entry(*id) {
                    std::collections::hash_map::Entry::Vacant(slot) => {
                        slot.insert(hint);
                    }
                    std::collections::hash_map::Entry::Occupied(slot) => {
                        if slot.get() != &hint {
                            return Err(crate::error::SoftDelayHintConflict {
                                id: *id,
                                hint_a: slot.get().clone(),
                                hint_b: hint,
                            }
                            .into());
                        }
                    }
                }
            }

            parsed_specs.insert((spec.id, obj.id), ext);
        }
    }
    let ext_specs = parsed_specs;

    // Transform flat list of extension refs into a HashMap indexed by their id
    let ext_ref_count = ext_refs.len();
    let ext_refs: HashMap<u32, _> = ext_refs.iter().map(|ext| (ext.id, *ext)).collect();
    if ext_refs.len() < ext_ref_count {
        return Err(ConversionError::EventIdReuse(
            raw::ExtensionRef::SECTION_TYPE,
        ));
    }

    // Each ref starts a linked list of extensions, terminated by `next == 0`.
    // Walk every chain, until the end is reached or a cycle detected.
    // Build a HashMap with key=ref_id, value=Vec<Extension>
    let mut parsed = HashMap::with_capacity(ext_refs.len());

    for root_ref in ext_refs.values() {
        // Keep a list of visited references - if we see one again we have a cycle
        let mut visited: HashSet<u32> = HashSet::new();
        // Convert the linked list of references into a Vec<Extensions>
        let mut ext_list = Vec::new();

        let mut ext_ref = root_ref;
        loop {
            // Return error on cycle
            if !visited.insert(ext_ref.id) {
                return Err(ConversionError::ExtensionRefCycle {
                    start_id: root_ref.id,
                });
            }

            // Convert the reference into a parsed extension and push to Vec
            let ext = ext_specs
                .get(&(ext_ref.spec_id, ext_ref.obj_id))
                .ok_or(ConversionError::InvalidExtensionRef { id: ext_ref.id })?;
            ext_list.push(ext.clone());

            // Break on end-of-linked-list
            if ext_ref.next == 0 {
                break;
            }
            // Otherwise move on to next element
            ext_ref = ext_refs
                .get(&ext_ref.next)
                .ok_or(ConversionError::InvalidExtensionRef { id: ext_ref.next })?;
        }
        parsed.insert(root_ref.id, ext_list);
    }
    Ok((parsed, soft_delay_hints))
}

fn convert_block(
    block: raw::Block,
    rfs: &HashMap<u32, Arc<seq::Rf>>,
    gradients: &HashMap<u32, Arc<seq::Gradient>>,
    adcs: &HashMap<u32, Arc<seq::Adc>>,
    delays: &HashMap<u32, f64>,
    time_raster: &seq::TimeRaster,
    exts: &HashMap<u32, Vec<seq::Extension>>,
) -> Result<seq::Block, ConversionError> {
    let err = |ty, id| ConversionError::BrokenRef { ty, id };
    use super::EventType::*;

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
        raw::BlockDuration::Duration(dur) => dur as f64 * time_raster.block,
        raw::BlockDuration::DelayId(delay) => {
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

    let ext = if block.ext != 0 {
        exts.get(&block.ext)
            .cloned()
            .ok_or(ConversionError::InvalidExtensionRef { id: block.ext })?
    } else {
        Vec::new()
    };

    Ok(seq::Block {
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

/// Active duration of a raw Free gradient's shape, in raster ticks (i.e.
/// "time units of the gradient raster"). Multiply by `time_raster.grad` for
/// seconds.
fn raw_shape_duration_ticks(
    shape_id: u32,
    time_id: i32,
    raw_shapes: &HashMap<u32, Arc<Vec<f64>>>,
) -> Result<f64, ConversionError> {
    let raw = raw_shapes
        .get(&shape_id)
        .ok_or(ConversionError::ShapeNotFound(shape_id))?;
    match time_id {
        0 => Ok(raw.len() as f64),
        -1 => Ok(raw.len().div_ceil(2) as f64),
        x if x > 0 => {
            let time_raw = raw_shapes
                .get(&(x as u32))
                .ok_or(ConversionError::ShapeNotFound(x as u32))?;
            time_raw.last().copied().ok_or(ConversionError::EmptyShape)
        }
        other => Err(ConversionError::UnknownTimeId(other)),
    }
}

/// Walk raw blocks in file order to compute `(first, last)` in absolute Hz/m
/// for every Free gradient id, matching the pypulseq convention:
///
/// - On each channel (gx/gy/gz) track the running `prev_last`, initialised to
///   `0.0`.
/// - For a Free gradient, `first = raw.first.unwrap_or(prev_last[ch])` and
///   `last = raw.last.unwrap_or(amp * extrapolated_last_norm)`. The result is
///   cached on first encounter and reused for every later block (frozen — same
///   `Arc<seq::Gradient>` is shared).
/// - After a Free gradient: `prev_last[ch] = last` if the gradient reaches the
///   block boundary, else `0.0` (next block starts at zero on this channel).
/// - After a Trap or empty channel: `prev_last[ch] = 0.0` (traps ramp back to
///   zero by their end, and empty channels imply a gap).
///
/// Pulseq 1.5+ gradients already carry `raw.first`/`raw.last`, so the unwrap
/// branch is skipped in that case.
fn compute_grad_boundaries(
    raw_blocks: &[raw::Block],
    raw_grad_map: &HashMap<u32, &raw::Gradient>,
    raw_trap_map: &HashMap<u32, &raw::Trap>,
    raw_shapes: &HashMap<u32, Arc<Vec<f64>>>,
    time_raster: &seq::TimeRaster,
) -> Result<HashMap<u32, (f64, f64)>, ConversionError> {
    let mut cache: HashMap<u32, (f64, f64)> = HashMap::new();
    let mut prev_last: [f64; 3] = [0.0; 3];

    for block in raw_blocks {
        // Block duration in seconds. For pre-1.4 (DelayId) we skip the
        // boundary check rather than recompute max-event-duration in raw
        // form — pre-1.4 sequences predate continuous-gradient practice, so
        // resetting `prev_last` to 0 between blocks is a safe approximation.
        let block_dur = match block.dur {
            raw::BlockDuration::Duration(dur) => Some(dur as f64 * time_raster.block),
            raw::BlockDuration::DelayId(_) => None,
        };

        for (ch, grad_id) in [(0, block.gx), (1, block.gy), (2, block.gz)] {
            if grad_id == 0 {
                prev_last[ch] = 0.0;
                continue;
            }

            if let Some(&grad) = raw_grad_map.get(&grad_id) {
                let (_first_abs, last_abs) = if let Some(&cached) = cache.get(&grad_id) {
                    cached
                } else {
                    let first = grad.first.unwrap_or(prev_last[ch]);
                    let last = match grad.last {
                        Some(l) => l,
                        None => {
                            let raw = raw_shapes
                                .get(&grad.shape_id)
                                .ok_or(ConversionError::ShapeNotFound(grad.shape_id))?;
                            let (_, last_norm) =
                                crate::raw::helpers::extrapolate_grad_boundaries(raw, grad.time_id);
                            grad.amp * last_norm
                        }
                    };
                    cache.insert(grad_id, (first, last));
                    (first, last)
                };

                // Update prev_last for the next block.
                let grad_dur = grad.delay
                    + raw_shape_duration_ticks(grad.shape_id, grad.time_id, raw_shapes)?
                        * time_raster.grad;
                let reaches_boundary = match block_dur {
                    Some(bd) => (grad_dur - bd).abs() < 0.5 * time_raster.grad,
                    None => false,
                };
                prev_last[ch] = if reaches_boundary { last_abs } else { 0.0 };
            } else if raw_trap_map.contains_key(&grad_id) {
                // Trap always ends at zero on its own; whether it reaches the
                // block boundary or not, prev_last on this channel is 0.
                prev_last[ch] = 0.0;
            }
            // else: dangling id — convert_block will report BrokenRef later.
        }
    }

    Ok(cache)
}
