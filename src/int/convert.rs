use std::collections::HashMap;
use std::sync::Arc;

use num_complex::Complex64;

use crate::error::{InterpreterError, InterpreterWarning};
use crate::int::{Quaternion, Transform};
use crate::seq;

/// Lowers a seq sequence into the int form, folding the relative
/// (`rel × larmor`) and absolute components of RF/ADC frequency and phase,
/// resolving RF shims from either the `rf_shims` extension or the pTx shim
/// shape on the seq RF, and applying the per-axis scale of `fov` to gradient
/// amplitudes.
pub fn convert(
    seq: &seq::Sequence,
    fov: super::Transform,
    larmor: f64,
    soft_delays: HashMap<String, f64>,
    warnings: &mut Vec<InterpreterWarning>,
) -> Result<super::Sequence, InterpreterError> {
    // Update the (purely informative) sequence FOV to account for scaling:
    let out_fov = {
        let s = fov.scale;
        seq.fov.map_or([s; 3], |(x, y, z)| [s * x, s * y, s * z])
    };

    // Track the channel count established by the first explicit shim so we
    // can warn (not error) if later RFs disagree.
    let mut expected_shim_channels: Option<usize> = None;
    // Sticky label state - all counters and flags start at 0 / false. Each
    // block's LABELSETs are applied before its LABELINCs (per spec). State
    // is then snapshotted: ADC-relevant fields go onto `Adc::labels`, the
    // block-level subset (trid, once, pmc, no_*) onto `Block`.
    let mut label_state = LabelState::default();
    // Memoizes seq → int shape conversions so blocks that share an
    // `Arc<seq::Shape>` end up sharing a single `Arc<int::Shape>` too.
    let mut shapes = ShapeLib::default();
    let mut blocks = Vec::with_capacity(seq.blocks.len());

    for block in &seq.blocks {
        let rf = block
            .rf
            .as_ref()
            .map(|rf| -> Result<super::Rf, InterpreterError> {
                let shims = resolve_shims(block.id, rf, &block.ext)?;
                if shims.len() > 1 {
                    match expected_shim_channels {
                        None => expected_shim_channels = Some(shims.len()),
                        Some(expected) if expected != shims.len() => {
                            warnings.push(InterpreterWarning::InconsistentShimChannelCount {
                                block_id: block.id,
                                expected,
                                got: shims.len(),
                            });
                        }
                        _ => {}
                    }
                }
                Ok(convert_rf(
                    rf,
                    larmor,
                    seq.time_raster.rf,
                    shims,
                    &mut shapes,
                ))
            })
            .transpose()?;

        // Apply any `Delay` extensions on this block. Each one computes a
        // candidate duration `t_factor * x + t_offset` where `x` is the
        // user-supplied value for that hint. The candidate may shorten or
        // lengthen the block; negative candidates are rejected with a warning
        // and the block keeps its default duration.
        let delay_count = block
            .ext
            .iter()
            .filter(|e| matches!(e, seq::Extension::Delay { .. }))
            .count();
        if delay_count > 1 {
            warnings.push(InterpreterWarning::MultipleSoftDelays { block_id: block.id });
        }
        let mut duration = block.duration;
        for ext in &block.ext {
            if let seq::Extension::Delay {
                id,
                t_offset,
                t_factor,
            } = ext
            {
                // Without a user-supplied value the block keeps the duration
                // stored in the file, which the exporter (pypulseq / MATLAB
                // `makeSoftDelay`) sets to the soft delay's default duration.
                let x = match seq.soft_delay_hints.get(id) {
                    Some(hint) => match soft_delays.get(hint) {
                        Some(value) => *value,
                        None => {
                            warnings
                                .push(InterpreterWarning::MissingSoftDelay { hint: hint.clone() });
                            continue;
                        }
                    },
                    None => {
                        warnings.push(InterpreterWarning::MissingSoftDelayHint { id: *id });
                        continue;
                    }
                };
                let computed = t_factor * x + t_offset;
                if computed >= 0.0 {
                    duration = computed;
                } else {
                    warnings.push(InterpreterWarning::NegativeSoftDelay {
                        block_id: block.id,
                        computed,
                        block: block.duration,
                    });
                }
            }
        }

        // First pass: apply all LABELSETs.
        for ext in &block.ext {
            if let seq::Extension::LabelSet { flag, value } = ext {
                label_state.apply_set(flag, *value, block.id)?;
            }
        }
        // Second pass: apply all LABELINCs.
        for ext in &block.ext {
            if let seq::Extension::LabelInc { counter, value } = ext {
                label_state.apply_inc(counter, *value);
            }
        }

        // Compute gradient transform for current block, starting with rot ext.
        // Default quaternion `[1, 0, 0, 0]` is the identity rotation; any
        // `rotations` extension on the block overrides it. Two or more
        // instances on the same block are ambiguous and rejected.
        let mut rot_iter = block.ext.iter().filter_map(|e| match e {
            seq::Extension::Rotation { quat } => Some(Quaternion(*quat)),
            _ => None,
        });
        let rot_extension = rot_iter.next().unwrap_or_default();
        if rot_iter.next().is_some() {
            return Err(InterpreterError::MultipleRotationExtensions { block_id: block.id });
        }

        // This is the full scanner transform of the sequence (PMC missing)
        let mut transform = fov;
        transform.rotation = transform.rotation * rot_extension;
        // Narrow it down if the seq flags were set
        if label_state.no_rot {
            transform.rotation = rot_extension;
        }
        if label_state.no_scl {
            transform.scale = 1.0;
        }
        if label_state.no_pos {
            transform.position = [0.0; 3];
        }

        let (gx, gy, gz) = transform_grad(
            block.gx.as_deref(),
            block.gy.as_deref(),
            block.gz.as_deref(),
            transform,
            seq.time_raster.grad,
            &mut shapes,
        );

        blocks.push(super::Block {
            duration,
            rf,
            gx,
            gy,
            gz,
            adc: block
                .adc
                .as_ref()
                .map(|adc| convert_adc(adc, larmor, label_state.adc_labels, &mut shapes)),
            triggers: block
                .ext
                .iter()
                .filter_map(|ext| match ext {
                    seq::Extension::Trigger {
                        typ,
                        channel,
                        delay,
                        duration,
                    } => Some(super::Trigger {
                        typ: *typ,
                        channel: *channel,
                        delay: *delay,
                        duration: *duration,
                    }),
                    _ => None,
                })
                .collect(),
            labels: label_state.block_labels,
        });
    }

    Ok(super::Sequence {
        name: seq.name.clone(),
        fov: out_fov,
        blocks,
    })
}

/// Resolves the shim for one RF. Errors on multiple `Shimming` extensions,
/// conflicting sources, or an empty explicit shim.
fn resolve_shims(
    block_id: u32,
    rf: &seq::Rf,
    extensions: &[seq::Extension],
) -> Result<Vec<Complex64>, InterpreterError> {
    let mut ext_iter = extensions.iter().filter_map(|e| match e {
        seq::Extension::Shimming { shim } => Some(shim),
        _ => None,
    });
    let ext_shim = ext_iter.next();
    if ext_iter.next().is_some() {
        return Err(InterpreterError::MultipleShimmingExtensions { block_id });
    }

    match (ext_shim, rf.shim_shape.as_ref()) {
        (Some(_), Some(_)) => Err(InterpreterError::ConflictingShimSources { block_id }),
        (Some(ext), None) => {
            if ext.is_empty() {
                return Err(InterpreterError::EmptyShim { block_id });
            }
            let shim = ext
                .iter()
                .map(|[a, p]| Complex64::from_polar(*a, p * std::f64::consts::TAU))
                .collect();
            Ok(shim)
        }
        (None, Some(ptx)) => {
            if ptx.amp.is_empty() {
                return Err(InterpreterError::EmptyShim { block_id });
            }
            Ok(ptx.amp.clone())
        }
        (None, None) => Ok(vec![Complex64::new(1.0, 0.0)]),
    }
}

fn convert_rf(
    rf: &seq::Rf,
    larmor: f64,
    rf_raster: f64,
    shims: Vec<Complex64>,
    shapes: &mut ShapeLib,
) -> super::Rf {
    super::Rf {
        amp: rf.amp,
        phase: rf.phase.0 * larmor + rf.phase.1,
        delay: rf.delay,
        center: rf.center,
        freq: rf.freq.0 * larmor + rf.freq.1,
        shape: shapes.get_complex(&rf.shape, rf_raster),
        shims,
        rf_use: rf.rf_use,
    }
}

/// Resolves a single seq gradient to its `(amp, delay, int_shape)` triple.
/// Trap and Free both end up with a memoized `Arc<int::Shape<f64>>`; for
/// Trap that's the synthesised `[0, 1, 1, 0]` envelope, for Free it's the
/// per-sample shape from the raw file.
fn lookup_grad(
    g: &seq::Gradient,
    grad_raster: f64,
    shapes: &mut ShapeLib,
) -> (f64, f64, Arc<super::Shape<f64>>) {
    match g {
        seq::Gradient::Free { amp, delay, shape } => (*amp, *delay, shapes.get(shape, grad_raster)),
        seq::Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => (*amp, *delay, shapes.get_trap(*rise, *flat, *fall)),
    }
}

type GradLookup = (f64, f64, Arc<super::Shape<f64>>);

/// Applies the FOV transform (scale + rotation) across all three gradient
/// axes at once. With identity rotation each output axis is independent and
/// the result matches per-axis scaling. With a non-identity rotation, inputs
/// sharing the same `int::Shape` Arc (after `ShapeLib` lookup) and `delay`
/// are mixed by amplitude only, keeping the shared shape. Otherwise all axes
/// are resampled onto a common time grid and rotated per sample (see
/// `rotate_resampled`). `None` axes can become `Some` if rotation projects
/// onto them.
fn transform_grad(
    gx: Option<&seq::Gradient>,
    gy: Option<&seq::Gradient>,
    gz: Option<&seq::Gradient>,
    transform: Transform,
    grad_raster: f64,
    shapes: &mut ShapeLib,
) -> (
    Option<super::Gradient>,
    Option<super::Gradient>,
    Option<super::Gradient>,
) {
    let lookups: [Option<GradLookup>; 3] = [
        gx.map(|g| lookup_grad(g, grad_raster, shapes)),
        gy.map(|g| lookup_grad(g, grad_raster, shapes)),
        gz.map(|g| lookup_grad(g, grad_raster, shapes)),
    ];

    // Identity rotation: each axis is independent, so we can sidestep the
    // shared-shape/delay requirement that the rotated path needs. Just apply
    // the inverse scale (`to_grad_transform` would do the same, but only
    // along the diagonal).
    if transform.rotation.is_identity() {
        let inv_scale = 1.0 / transform.scale;
        let emit = |opt: Option<GradLookup>| {
            opt.map(|(amp, delay, shape)| super::Gradient {
                amp: amp * inv_scale,
                delay,
                shape,
            })
        };
        let [lx, ly, lz] = lookups;
        return (emit(lx), emit(ly), emit(lz));
    }

    // Pick the first present axis as the reference. If nothing is present
    // there's no gradient — rotation has nothing to project from.
    let Some((_, ref_delay, ref_shape)) = lookups.iter().find_map(|opt| opt.as_ref()).cloned()
    else {
        return (None, None, None);
    };

    let m = transform.to_grad_transform();

    // If all present axes agree on shape and delay, rotation only mixes the
    // amplitudes and the shared shape can be kept. Arc::ptr_eq catches
    // Trap+Free mixes, mismatched trap timings, and different free shapes
    // uniformly because ShapeLib gives the same Arc only for inputs that
    // share both kind and parameters.
    let shared = lookups
        .iter()
        .flatten()
        .all(|(_, delay, shape)| *delay == ref_delay && Arc::ptr_eq(shape, &ref_shape));
    if !shared {
        return rotate_resampled(&lookups, m);
    }

    let amps = lookups
        .each_ref()
        .map(|opt| opt.as_ref().map_or(0.0, |(a, _, _)| *a));
    let out = mat_vec(m, amps);

    let emit = |amp: f64| {
        if amp == 0.0 {
            None
        } else {
            Some(super::Gradient {
                amp,
                delay: ref_delay,
                shape: ref_shape.clone(),
            })
        }
    };
    (emit(out[0]), emit(out[1]), emit(out[2]))
}

fn mat_vec(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// Two grid points closer than this `[s]` are merged into one. Shape times
/// and delays are raster multiples computed in floating point, so the same
/// instant can arrive with slightly different bits from different axes.
const GRID_EPS: f64 = 1e-12;

/// Rotation path for axes that don't share shape and delay. Each present
/// axis is a piecewise-linear waveform `amp * shape(t - delay)` on
/// `[delay, delay + duration]` and zero outside. The union of all
/// breakpoints (samples and support edges) is a grid on which every input
/// is linear between neighbours, so rotating the samples on that grid and
/// linearly interpolating the result is exact.
///
/// Each output axis gets its own freshly built shape normalized to a peak
/// magnitude of 1, with the peak (signed) going into `amp`. The common
/// `delay` is the earliest input delay. Output axes that are zero everywhere
/// are dropped.
fn rotate_resampled(
    lookups: &[Option<GradLookup>; 3],
    m: [[f64; 3]; 3],
) -> (
    Option<super::Gradient>,
    Option<super::Gradient>,
    Option<super::Gradient>,
) {
    let present = || lookups.iter().flatten();
    let start = present()
        .map(|(_, delay, _)| *delay)
        .fold(f64::INFINITY, f64::min);
    let end = present()
        .map(|(_, delay, shape)| delay + shape.duration)
        .fold(f64::NEG_INFINITY, f64::max);
    if !start.is_finite() {
        return (None, None, None);
    }

    // Grid relative to `start`.
    let mut grid: Vec<f64> = present()
        .flat_map(|(_, delay, shape)| {
            let offset = delay - start;
            shape
                .time
                .iter()
                .map(move |t| offset + t)
                .chain([offset, offset + shape.duration])
        })
        .collect();
    grid.sort_by(f64::total_cmp);
    grid.dedup_by(|b, a| *b - *a <= GRID_EPS);

    // Waveform of one input axis at grid time `t` (relative to `start`).
    let sample = |opt: &Option<GradLookup>, t: f64| -> f64 {
        let Some((amp, delay, shape)) = opt else {
            return 0.0;
        };
        let local = t - (delay - start);
        if local < -GRID_EPS || local > shape.duration + GRID_EPS {
            0.0
        } else {
            amp * shape.interpolate(local)
        }
    };

    let mut out: [Vec<f64>; 3] = Default::default();
    for &t in &grid {
        let rotated = mat_vec(m, lookups.each_ref().map(|opt| sample(opt, t)));
        for (axis, value) in out.iter_mut().zip(rotated) {
            axis.push(value);
        }
    }

    // `end - start` and the grid's last point are computed differently and
    // may disagree by an ulp; the shape invariant needs `time <= duration`.
    let duration = grid.last().map_or(end - start, |&t| t.max(end - start));
    let emit = |values: Vec<f64>| -> Option<super::Gradient> {
        let peak = values
            .iter()
            .copied()
            .max_by(|a, b| a.abs().total_cmp(&b.abs()))?;
        if peak == 0.0 {
            return None;
        }
        Some(super::Gradient {
            amp: peak,
            delay: start,
            shape: Arc::new(super::Shape {
                time: grid.clone(),
                amp: values.into_iter().map(|v| v / peak).collect(),
                duration,
            }),
        })
    };
    let [ox, oy, oz] = out;
    (emit(ox), emit(oy), emit(oz))
}

fn convert_adc(
    adc: &seq::Adc,
    larmor: f64,
    labels: super::Labels,
    shapes: &mut ShapeLib,
) -> super::Adc {
    super::Adc {
        num: adc.num,
        dwell: adc.dwell,
        delay: adc.delay,
        freq: adc.freq.0 * larmor + adc.freq.1,
        phase: adc.phase.0 * larmor + adc.phase.1,
        // ADC phase shapes are sampled per-ADC-sample at `dwell`, so we
        // multiply the seq tick-domain time by `dwell` to get seconds.
        phase_shape: adc.phase_shape.as_ref().map(|s| shapes.get(s, adc.dwell)),
        labels,
    }
}

// TODO: might be worth it to write a generic shape lib shared by raw->seq and
// seq->int conversion?
//
/// Memoizes seq → int shape conversions so repeated references to the same
/// `Arc<seq::Shape>` produce a single shared `Arc<int::Shape>`. Mirrors the
/// role of `seq::convert::ShapeLib` at the previous stage.
///
/// Cache key is `(Arc::as_ptr as usize, raster.to_bits())`: pointer identity
/// of the input shape combined with the raster used to lift it into seconds.
/// The raster is part of the key because ADC phase shapes are converted with
/// `dwell`, which can differ per ADC even when the same seq shape is reused.
/// Pointer reuse can't happen during conversion because the seq `Sequence`
/// (which owns those shapes) outlives this function.
#[derive(Default)]
struct ShapeLib {
    real: HashMap<(usize, u64), Arc<super::Shape<f64>>>,
    complex: HashMap<(usize, u64), Arc<super::Shape<Complex64>>>,
    /// Synthesised trapezoid envelopes: `time = [0, rise, rise+flat,
    /// rise+flat+fall]`, `amp = [0, 1, 1, 0]`. Cached by `(rise, flat, fall)`
    /// — `delay` and `amp` belong to the gradient, not the shape, so
    /// dropping them from the key maximises sharing across blocks that
    /// reuse a trap timing.
    trap: HashMap<(u64, u64, u64), Arc<super::Shape<f64>>>,
}

impl ShapeLib {
    fn get(&mut self, shape: &Arc<seq::Shape<f64>>, raster: f64) -> Arc<super::Shape<f64>> {
        let key = (Arc::as_ptr(shape) as usize, raster.to_bits());
        self.real
            .entry(key)
            .or_insert_with(|| {
                Arc::new(super::Shape {
                    time: shape.time.iter().map(|&t| t * raster).collect(),
                    amp: shape.amp.clone(),
                    duration: shape.duration as f64 * raster,
                })
            })
            .clone()
    }

    fn get_complex(
        &mut self,
        shape: &Arc<seq::Shape<Complex64>>,
        raster: f64,
    ) -> Arc<super::Shape<Complex64>> {
        let key = (Arc::as_ptr(shape) as usize, raster.to_bits());
        self.complex
            .entry(key)
            .or_insert_with(|| {
                Arc::new(super::Shape {
                    time: shape.time.iter().map(|&t| t * raster).collect(),
                    amp: shape.amp.clone(),
                    duration: shape.duration as f64 * raster,
                })
            })
            .clone()
    }

    fn get_trap(&mut self, rise: f64, flat: f64, fall: f64) -> Arc<super::Shape<f64>> {
        let key = (rise.to_bits(), flat.to_bits(), fall.to_bits());
        self.trap
            .entry(key)
            .or_insert_with(|| {
                Arc::new(super::Shape {
                    time: vec![0.0, rise, rise + flat, rise + flat + fall],
                    amp: vec![0.0, 1.0, 1.0, 0.0],
                    duration: rise + flat + fall,
                })
            })
            .clone()
    }
}

#[derive(Default)]
struct LabelState {
    adc_labels: super::Labels,
    block_labels: super::BlockLabels,

    /// Disable FOV rotations for the current block
    no_rot: bool,
    /// Disable FOV positioning for the current block
    no_pos: bool,
    /// Disable FOV scaling for the current block
    no_scl: bool,
}

impl LabelState {
    fn apply_set(
        &mut self,
        flag: &seq::extensions::ExtLabelFlag,
        value: i32,
        block_id: u32,
    ) -> Result<(), InterpreterError> {
        use seq::extensions::ExtLabelFlag as F;
        // Counter and ONCE accept any i32; everything else must be 0 or 1.
        match flag {
            F::Counter(c) => {
                *self.counter_mut(c) = value;
                return Ok(());
            }
            F::Once => {
                self.block_labels.once = match value {
                    0 => super::Once::Always,
                    1 => super::Once::First,
                    _ => super::Once::Last,
                };
                return Ok(());
            }
            _ => {}
        }
        let on = match value {
            0 => false,
            1 => true,
            _ => {
                return Err(InterpreterError::FlagSetNonBoolean {
                    block_id,
                    flag: flag.to_string(),
                    value,
                });
            }
        };
        match flag {
            F::Nav => self.adc_labels.nav = on,
            F::Rev => self.adc_labels.rev = on,
            F::Sms => self.adc_labels.sms = on,
            F::Ref => self.adc_labels.ref_ = on,
            F::Ima => self.adc_labels.ima = on,
            F::Off => self.adc_labels.off = on,
            F::Noise => self.adc_labels.noise = on,
            F::Pmc => self.block_labels.pmc = on,
            F::NoRot => self.no_rot = on,
            F::NoPos => self.no_pos = on,
            F::NoScl => self.no_scl = on,
            // Counters / Once handled above by early-return.
            F::Counter(_) | F::Once => unreachable!(),
        }
        Ok(())
    }

    fn apply_inc(&mut self, counter: &seq::extensions::ExtLabelCounter, value: i32) {
        let target = self.counter_mut(counter);
        *target = target.wrapping_add(value);
    }

    fn counter_mut(&mut self, counter: &seq::extensions::ExtLabelCounter) -> &mut i32 {
        use seq::extensions::ExtLabelCounter as C;
        match counter {
            C::Slc => &mut self.adc_labels.slc,
            C::Seg => &mut self.adc_labels.seg,
            C::Rep => &mut self.adc_labels.rep,
            C::Avg => &mut self.adc_labels.avg,
            C::Set => &mut self.adc_labels.set,
            C::Eco => &mut self.adc_labels.eco,
            C::Phs => &mut self.adc_labels.phs,
            C::Lin => &mut self.adc_labels.lin,
            C::Par => &mut self.adc_labels.par,
            C::Acq => &mut self.adc_labels.acq,
            C::Trid => &mut self.block_labels.trid,
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn trap(amp: f64, rise: f64, flat: f64, fall: f64, delay: f64) -> seq::Gradient {
        seq::Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        }
    }

    /// Evaluates an int gradient at absolute block time `t`.
    fn eval(g: Option<&super::super::Gradient>, t: f64) -> f64 {
        g.map_or(0.0, |g| {
            let local = t - g.delay;
            if local < 0.0 || local > g.shape.duration {
                0.0
            } else {
                g.amp * g.shape.interpolate(local)
            }
        })
    }

    fn rot_z(angle: f64) -> Transform {
        let h = angle / 2.0;
        Transform {
            rotation: Quaternion([h.cos(), 0.0, 0.0, h.sin()]),
            ..Default::default()
        }
    }

    #[test]
    fn rotation_with_different_shapes_and_delays() {
        let gx = trap(1000.0, 1e-4, 5e-4, 1e-4, 0.0);
        let gy = trap(-400.0, 2e-4, 1e-4, 3e-4, 2e-4);
        let transform = rot_z(0.3);
        let m = transform.to_grad_transform();
        let mut shapes = ShapeLib::default();
        let (ox, oy, oz) = transform_grad(Some(&gx), Some(&gy), None, transform, 1e-5, &mut shapes);
        assert!(oz.is_none());

        let ix = lookup_grad(&gx, 1e-5, &mut shapes);
        let iy = lookup_grad(&gy, 1e-5, &mut shapes);
        let input = |(amp, delay, shape): &GradLookup, t: f64| {
            let local = t - delay;
            if local < 0.0 || local > shape.duration {
                0.0
            } else {
                amp * shape.interpolate(local)
            }
        };
        for i in 0..=1000 {
            let t = i as f64 * 1e-6;
            let v = [input(&ix, t), input(&iy, t), 0.0];
            let expected = mat_vec(m, v);
            assert!(
                (eval(ox.as_ref(), t) - expected[0]).abs() < 1e-9,
                "x at {t}"
            );
            assert!(
                (eval(oy.as_ref(), t) - expected[1]).abs() < 1e-9,
                "y at {t}"
            );
        }
    }

    #[test]
    fn rotation_with_shared_shape_keeps_it() {
        let gx = trap(1000.0, 1e-4, 5e-4, 1e-4, 0.0);
        let gy = trap(-400.0, 1e-4, 5e-4, 1e-4, 0.0);
        let mut shapes = ShapeLib::default();
        let (ox, oy, _) = transform_grad(Some(&gx), Some(&gy), None, rot_z(0.3), 1e-5, &mut shapes);
        assert!(Arc::ptr_eq(&ox.unwrap().shape, &oy.unwrap().shape));
    }
}
