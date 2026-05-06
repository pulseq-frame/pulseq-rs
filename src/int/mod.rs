use crate::error::{InterpreterError, InterpreterWarning};
use num_complex::Complex64;
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

mod convert;

pub struct Sequence {
    pub name: Option<String>,
    pub blocks: Vec<Block>,
}

impl Sequence {
    /// Parameters:
    /// - Field of view in `[m]`: applied as gradient scaling.
    /// - Larmor frequency `[Hz]`: for relative frequency / phase
    /// - Values for soft delays, keyed by their text id.
    pub fn from_seq(
        seq: &crate::seq::Sequence,
        fov: Option<[f64; 3]>,
        larmor: f64,
        soft_delays: HashMap<String, f64>,
    ) -> Result<(Self, Vec<InterpreterWarning>), InterpreterError> {
        // The sequence has a FOV and we have an optional FOV input.
        // If both are set and we have a mismatch scale accordingly.
        let seq_fov = seq.fov.map_or([1.0; 3], |(x, y, z)| [x, y, z]);
        let out_fov = fov.unwrap_or(seq_fov);
        let fov_scale = [
            out_fov[0] / seq_fov[0],
            out_fov[1] / seq_fov[1],
            out_fov[2] / seq_fov[2],
        ];

        let mut warnings = Vec::new();
        let seq = convert::convert(seq, fov_scale, larmor, soft_delays, &mut warnings)?;
        Ok((seq, warnings))
    }
}

pub struct Block {
    /// ID kept for error reporting and round-trip debugging.
    pub id: u32,
    /// `[s]`
    pub duration: f64,
    pub rf: Option<Arc<Rf>>,
    pub gx: Option<Arc<Gradient>>,
    pub gy: Option<Arc<Gradient>>,
    pub gz: Option<Arc<Gradient>>,
    pub adc: Option<Arc<Adc>>,
    /// Triggers from the `triggers` extension active in this block.
    pub triggers: Vec<Trigger>,
    /// Repetition gating from the `ONCE` label
    pub once: Once,
    /// `PMC` label - block can be prospectively motion-corrected.
    pub pmc: bool,
}

/// tells if block should be measured only in the first or last repetition
pub enum Once {
    Always,
    First,
    Last,
}

pub struct Trigger {
    pub typ: u32,
    pub channel: u32,
    /// `[s]`
    pub delay: f64,
    /// `[s]`
    pub duration: f64,
}

pub struct Rf {
    /// `[Hz]`
    pub amp: f64,
    /// `[rad]` - relative and offset components combined via the larmor frequency.
    pub phase: f64,
    /// `[s]`
    pub delay: f64,
    /// `[s]`
    pub center: f64,
    /// `[Hz]` - relative and offset components combined via the larmor frequency.
    pub freq: f64,
    /// Combined amplitude × exp(i × phase) base shape.
    pub shape: Arc<Shape<Complex64>>,
    /// Per-channel shim multipliers. `None` = single channel.
    /// Each entry is a per-sample shape; constant shims (from the official
    /// `rf_shims` extension) are stored as length-1 shapes, full pTx shapes
    /// (from the Martin pTx `shim_id` field) keep their per-sample resolution.
    pub shims: Option<Vec<Arc<Shape<Complex64>>>>,
    /// forwarded from raw sequence - specifies what purpose this pulse has.
    pub rf_use: crate::raw::RfUse,
}

pub enum Gradient {
    Free {
        /// `[Hz/m]` - already FOV-scaled and rotated.
        amp: f64,
        /// `[s]`
        delay: f64,
        shape: Arc<Shape<f64>>,
    },
    Trap {
        /// `[Hz/m]` - already FOV-scaled. Note: rotating a trapezoid around
        /// an arbitrary axis only stays a trapezoid when all three channels
        /// share timings, otherwise the rotation step lowers it to `Free`.
        amp: f64,
        /// `[s]`
        rise: f64,
        /// `[s]`
        flat: f64,
        /// `[s]`
        fall: f64,
        /// `[s]`
        delay: f64,
    },
}

pub struct Adc {
    pub num: u32,
    /// `[s]`
    pub dwell: f64,
    /// `[s]`
    pub delay: f64,
    /// `[Hz]` - relative and offset components combined via the larmor frequency.
    pub freq: f64,
    /// `[rad]` - relative and offset components combined via the larmor frequency.
    pub phase: f64,
    /// Optional per-sample phase modulation, applied on top of `phase`.
    pub phase_shape: Option<Arc<Shape<f64>>>,
    /// Snapshot of the label state at the time this ADC fires.
    pub labels: Labels,
}

/// Per-ADC label state. Counters reflect the running value at this ADC,
/// boolean flags are sticky until cleared by another `LABELSET`.
#[derive(Default, Clone, Copy)]
pub struct Labels {
    pub slc: i32,
    pub seg: i32,
    pub rep: i32,
    pub avg: i32,
    pub set: i32,
    pub eco: i32,
    pub phs: i32,
    pub lin: i32,
    pub par: i32,
    pub acq: i32,
    /// Marks the start of a repeatable module (TR boundary).
    pub trid: i32,
    pub nav: bool,
    pub rev: bool,
    pub sms: bool,
    /// `REF` flag (renamed - `ref` is a Rust keyword).
    pub is_ref: bool,
    pub ima: bool,
    pub noise: bool,
}

/// Sparse sample representation, mirroring `seq::Shape` but with `time` and
/// `duration` in seconds (already multiplied by the appropriate raster during
/// seq→int lowering). `int` no longer carries a `time_raster`, since every
/// shape already knows its own absolute timing.
///
/// Invariants: same as `seq::Shape`. `duration` is the total active extent in
/// seconds and may be larger than `*time.last()` (e.g. for shapes with samples
/// at centers `[0.5, 1.5, …, N-0.5] * raster`, duration is `N * raster`).
pub struct Shape<T> {
    /// Absolute times in seconds for each sample.
    pub time: Vec<f64>,
    /// Sample values aligned with `time` 1:1.
    pub amp: Vec<T>,
    /// Total active extent in seconds. Not necessarily `*time.last()`.
    pub duration: f64,
}

impl<T> Shape<T> {
    /// Validate invariants. Mirrors `seq::Shape::new` but with `f64` time and
    /// duration.
    pub fn new(time: Vec<f64>, amp: Vec<T>, duration: f64) -> Option<Self> {
        if time.len() != amp.len() || time.is_empty() {
            return None;
        }
        if !time.array_windows().all(|[w1, w2]| w1 < w2) {
            return None;
        }
        if time.iter().any(|&t| t < 0.0 || t > duration) {
            return None;
        }
        Some(Self {
            time,
            amp,
            duration,
        })
    }
}

impl<T> Shape<T>
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f64, Output = T>,
{
    /// Linear interpolation at `time` (in seconds). Returns `amp[0]` for
    /// `time <= time[0]` and `*amp.last()` for `time >= time.last()`.
    #[allow(clippy::indexing_slicing)]
    pub fn interpolate(&self, time: f64) -> T {
        if time <= self.time[0] {
            return self.amp[0];
        }
        let last = self.time.len() - 1;
        if time >= self.time[last] {
            return self.amp[last];
        }
        let idx = self.time.iter().position(|&t| t >= time).unwrap_or(last);
        let t0 = self.time[idx - 1];
        let t1 = self.time[idx];
        let frac = (time - t0) / (t1 - t0);
        self.amp[idx - 1] + (self.amp[idx] - self.amp[idx - 1]) * frac
    }
}
