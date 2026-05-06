// Interpretation diagnostics, grouped by the step that produces them.
// Severity is a recommendation only - flip an entry from warning to error if a
// caller wants stricter behaviour (or vice versa) by promoting it in the enum.
//
// Step 1 - FOV scaling
//   error:    fov component is zero, negative or NaN (would silently zero
//             gradients or produce non-finite amplitudes)
//   warning:  no_rot / no_scale flag toggled inside a block that already has
//             gradients (intent is ambiguous - applies from next block)
//
// Step 2 - ADC labels
//   error:    none expected; the parser/seq layer already rejects unknown
//             label names and non-counter LABELINC targets
//   warning:  a counter goes negative (legal per spec, but usually a bug)
//   warning:  LABELSET overrides a non-default value within the same block
//             (last-write-wins, but the earlier write is dead code)
//
// Step 3 - Soft delays
//   error:    a block references a soft delay whose `text_id` has no entry
//             in `Data::soft_delays`
//   error:    two soft-delay extension instances share a `text_id` but
//             disagree on `t_offset` / `t_factor` (spec violation)
//   error:    the resulting block duration is negative or non-finite
//   warning:  resulting duration is not a multiple of the block raster
//             (we round; caller may want to know)
//   warning:  `Data::soft_delays` contains keys that are never referenced
//
// Step 4 - Frequency / phase unification
//   error:    `Data::larmor` is zero, negative or NaN
//
// Step 5 - Rotation extension
//   warning:  quaternion is not unit length (we renormalise)
//   warning:  rotation forces a Trap to be lowered to a Free shape (caller
//             may care for hardware-specific reasons)
//   error:    quaternion contains NaN / Inf
//
// Step 6 - Once / Pmc / triggers
//   error:    ONCE label set to a value outside {0, 1, 2}
//   warning:  trigger `delay + duration` exceeds the block duration
//   warning:  multiple triggers on the same channel overlap in time
//
// Step 7 - Shim unification
//   error:    a single RF carries both an `rf_shims` extension and a pTx
//             `shim_id` with conflicting channel counts
//   error:    a pTx shim shape length differs from the RF shape length
//   warning:  channel count changes from one RF to the next (legal, but
//             usually indicates an authoring mistake)
//
// Cross-cutting
//   warning:  an `Extension::Unsupported` was encountered and dropped
//             (include the `string_id` so the caller can decide whether
//             that extension actually mattered)
//
// Reporting recommendation:
//   change the signature to
//       pub fn from_seq(seq: &seq::Sequence, data: Data)
//           -> Result<(Self, Vec<Warning>), InterpretError>
//   and add two enums next to the existing `error` module:
//   - `InterpretError`  - one variant per hard-error case above, wrapped by
//                         the crate-level `Error` like the other phases
//   - `Warning`         - one variant per soft-warning case, carrying the
//                         block id (and where relevant, the offending value)
//                         so callers can render a useful message
//   Returning warnings as a `Vec` rather than via a callback keeps `from_seq`
//   pure and lets the viewer/CLI choose how loud to be (print, ignore, or
//   promote to errors with `--strict`).

use std::collections::HashMap;
use std::sync::Arc;

use num_complex::Complex64;

use crate::error::{InterpreterError, InterpreterWarning};
use crate::seq;

/// Bare-minimum interpretation: copies the seq sequence into the int form,
/// applying FOV scaling on gradient amplitudes, folding the relative
/// (`rel × larmor`) and absolute components of RF/ADC frequency and phase,
/// and resolving RF shims from either the `rf_shims` extension or the pTx
/// shim shape on the seq RF.
///
/// Other extensions are ignored — triggers stay empty, `Once::Always`,
/// `pmc = false`, `Labels::default()`. Soft delays are ignored too; block
/// durations are taken verbatim from `seq`. Future steps fill these in.
///
/// `fov_scale` tells us by how much to increase / decrease FOV per axis (the
/// caller already worked out `out_fov / seq_fov`); a value of 2 means we
/// double FOV and halve the gradients.
pub fn convert(
    seq: &seq::Sequence,
    fov_scale: [f64; 3],
    larmor: f64,
    _soft_delays: HashMap<String, f64>,
    warnings: &mut Vec<InterpreterWarning>,
) -> Result<super::Sequence, InterpreterError> {
    // Track the channel count established by the first explicit shim so we
    // can warn (not error) if later RFs disagree.
    let mut expected_shim_channels: Option<usize> = None;
    let mut blocks = Vec::with_capacity(seq.blocks.len());

    for block in &seq.blocks {
        let rf = block
            .rf
            .as_ref()
            .map(|rf| -> Result<Arc<super::Rf>, InterpreterError> {
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
                Ok(convert_rf(rf, larmor, seq.time_raster.rf, shims))
            })
            .transpose()?;

        blocks.push(super::Block {
            id: block.id,
            duration: block.duration,
            rf,
            gx: block
                .gx
                .as_ref()
                .map(|g| convert_grad(g, fov_scale[0], seq.time_raster.grad)),
            gy: block
                .gy
                .as_ref()
                .map(|g| convert_grad(g, fov_scale[1], seq.time_raster.grad)),
            gz: block
                .gz
                .as_ref()
                .map(|g| convert_grad(g, fov_scale[2], seq.time_raster.grad)),
            adc: block.adc.as_ref().map(|adc| convert_adc(adc, larmor)),
            triggers: Vec::new(),
            once: super::Once::Always,
            pmc: false,
        });
    }

    Ok(super::Sequence {
        name: seq.name.clone(),
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
) -> Arc<super::Rf> {
    Arc::new(super::Rf {
        amp: rf.amp,
        phase: rf.phase.0 * larmor + rf.phase.1,
        delay: rf.delay,
        center: rf.center,
        freq: rf.freq.0 * larmor + rf.freq.1,
        shape: convert_shape(&rf.shape, rf_raster),
        shims,
        rf_use: rf.rf_use,
    })
}

fn convert_grad(g: &seq::Gradient, fov_scale: f64, grad_raster: f64) -> Arc<super::Gradient> {
    Arc::new(match g {
        seq::Gradient::Free { amp, delay, shape } => super::Gradient::Free {
            amp: amp / fov_scale,
            delay: *delay,
            shape: convert_shape(shape, grad_raster),
        },
        seq::Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => super::Gradient::Trap {
            amp: amp / fov_scale,
            rise: *rise,
            flat: *flat,
            fall: *fall,
            delay: *delay,
        },
    })
}

fn convert_adc(adc: &seq::Adc, larmor: f64) -> Arc<super::Adc> {
    Arc::new(super::Adc {
        num: adc.num,
        dwell: adc.dwell,
        delay: adc.delay,
        freq: adc.freq.0 * larmor + adc.freq.1,
        phase: adc.phase.0 * larmor + adc.phase.1,
        // ADC phase shapes are sampled per-ADC-sample at `dwell`, so we
        // multiply the seq tick-domain time by `dwell` to get seconds.
        phase_shape: adc
            .phase_shape
            .as_ref()
            .map(|s| convert_shape(s, adc.dwell)),
        labels: super::Labels::default(),
    })
}

fn convert_shape<T: Clone>(shape: &seq::Shape<T>, raster: f64) -> Arc<super::Shape<T>> {
    Arc::new(super::Shape {
        time: shape.time.iter().map(|&t| t * raster).collect(),
        amp: shape.amp.clone(),
        duration: shape.duration as f64 * raster,
    })
}
