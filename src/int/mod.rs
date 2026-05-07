use crate::error::{InterpreterError, InterpreterWarning};
use num_complex::Complex64;
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

mod convert;

pub struct Sequence {
    pub name: Option<String>,
    /// Per-axis FOV `[m]` - based on [1, 1, 1] if .seq file did not define FOV
    pub fov: [f64; 3],
    pub blocks: Vec<Block>,
}

impl Sequence {
    /// Parameters:
    /// - `fov`: 3x4 affine transform applied to the sequence, must be unitary.
    /// - `larmor`: Larmor frequency `[Hz]` - used for relative freq/phase.
    /// - `soft_delays`: values for soft delays, keyed by their hint string.
    pub fn from_seq(
        seq: &crate::seq::Sequence,
        fov: Fov,
        larmor: f64,
        soft_delays: HashMap<String, f64>,
    ) -> Result<(Self, Vec<InterpreterWarning>), InterpreterError> {
        if !fov.validate() {
            return Err(InterpreterError::NonUnitaryFov);
        }

        let mut warnings = Vec::new();
        let seq = convert::convert(seq, fov, larmor, soft_delays, &mut warnings)?;
        Ok((seq, warnings))
    }
}

// Quaternions and FOV deserve their own math module with tests
#[derive(Clone, Copy)]
pub struct Quaternion(pub [f64; 4]);

impl Default for Quaternion {
    fn default() -> Self {
        Self([1.0, 0.0, 0.0, 0.0])
    }
}

impl Mul for Quaternion {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let x = self.0;
        let y = rhs.0;
        Self([
            x[0] * y[0] - x[1] * y[1] - x[2] * y[2] - x[3] * y[3],
            x[0] * y[1] + x[1] * y[0] + x[2] * y[3] - x[3] * y[2],
            x[0] * y[2] - x[1] * y[3] + x[2] * y[0] + x[3] * y[1],
            x[0] * y[3] + x[1] * y[2] - x[2] * y[1] + x[3] * y[0],
        ])
    }
}

impl Quaternion {
    pub fn norm(&self) -> f64 {
        (self.0[0].powi(2) + self.0[1].powi(2) + self.0[2].powi(2) + self.0[3].powi(2)).sqrt()
    }

    pub fn is_unit(&self) -> bool {
        const EPS: f64 = 1e-9;
        (1.0 - self.norm()).abs() <= EPS
    }
}

#[derive(Clone, Copy)]
pub struct Fov {
    pub scale: f64,
    pub rotation: Quaternion,
    /// Position will be ignored by the interpreter until I know how to apply it
    pub position: [f64; 3],
}

impl Default for Fov {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: Quaternion::default(),
            position: [0.0; 3],
        }
    }
}

impl Fov {
    /// converts the user defined FOV into a 3x3 transformation matrix that can
    /// be applied directly onto gradients to realize the scaling / rotation.
    /// This means inverting the scale *and the rotation!*
    pub fn to_grad_transform(self) -> [[f64; 3]; 3] {
        let q = self.rotation.0;
        let q = [q[0], -q[1], -q[2], -q[3]];
        let s = 1.0 / self.scale;
        [
            [
                s * (1.0 - 2.0 * (q[2].powi(2) + q[3].powi(2))),
                s * (2.0 * (q[1] * q[2] - q[0] * q[3])),
                s * (2.0 * (q[1] * q[3] + q[0] * q[2])),
            ],
            [
                s * (2.0 * (q[1] * q[2] + q[0] * q[3])),
                s * (1.0 - 2.0 * (q[1].powi(2) + q[3].powi(2))),
                s * (2.0 * (q[2] * q[3] - q[0] * q[1])),
            ],
            [
                s * (2.0 * (q[1] * q[3] - q[0] * q[2])),
                s * (2.0 * (q[2] * q[3] + q[0] * q[1])),
                s * (1.0 - 2.0 * (q[1].powi(2) + q[2].powi(2))),
            ],
        ]
    }

    /// Returns `true` iff the 3x3 part is a uniformly-scaled rotation /
    /// reflection — orthogonal columns with all three column norms equal.
    /// The common scale `s = ||c_i||` may be any positive finite value.
    /// Pure rotation (`s == 1`) passes; non-uniform scale, shear,
    /// non-orthogonal columns, and a zero-scale degenerate matrix all fail.
    #[allow(clippy::indexing_slicing)]
    pub fn validate(&self) -> bool {
        0.0 < self.scale
            && self.scale.is_finite()
            && self.position[0].is_finite()
            && self.position[1].is_finite()
            && self.position[2].is_finite()
            && self.rotation.is_unit()
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
    /// Label state from the `labelset` / `labelinc` extension.
    pub labels: BlockLabels,
}

#[derive(Default, Clone, Copy)]
pub struct BlockLabels {
    /// Repetition gating from the `ONCE` label.
    pub once: Once,
    /// `PMC` label - block can be prospectively motion-corrected.
    pub pmc: bool,
    /// `TRID` counter - marks the start (and identity) of a repeated seq part
    pub trid: i32,
}

/// tells if block should be measured only in the first or last repetition
#[derive(Default, Clone, Copy)]
pub enum Once {
    #[default]
    Always = 0,
    First = 1,
    Last = 2,
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
    /// Per-channel shim weights, one complex value per transmit channel.
    /// Sourced from either the official `rf_shims` extension on the block or
    /// the pTx (Martin) shim shape attached to the seq RF.
    /// A missing shim is represented as `vec![Complex64::new(1.0, 0.0)]`.
    pub shims: Vec<Complex64>,
    /// forwarded from raw sequence - specifies what purpose this pulse has.
    pub rf_use: crate::raw::RfUse,
}

/// Remove type distinction - Trap gradients are converted to Free with time shape
pub struct Gradient {
    /// `[Hz/m]` - already FOV-scaled and rotated.
    pub amp: f64,
    /// `[s]`
    pub delay: f64,
    pub shape: Arc<Shape<f64>>,
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
    pub nav: bool,
    pub rev: bool,
    pub sms: bool,
    pub ref_: bool,
    pub ima: bool,
    pub off: bool,
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
