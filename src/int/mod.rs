use std::collections::HashMap;
use std::sync::Arc;

use crate::seq::{self, ComplexShape, Shape};

pub use crate::raw::RfUse;

pub struct Sequence {
    pub name: Option<String>,
    pub blocks: Vec<Block>,
}

pub struct Data {
    /// Field of view in `[m]` - applied as gradient scaling.
    pub fov: [f64; 3],
    /// Larmor frequency `[Hz]` - used to fold the relative frequency / phase
    /// (which scale with B0) into the absolute offsets.
    pub larmor: f64,
    /// Values for soft delays, keyed by their text id.
    pub soft_delays: HashMap<String, f64>,
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
    /// Repetition gating from the `ONCE` label. `None` = run on every rep.
    pub once: Option<Once>,
    /// `PMC` label - block can be prospectively motion-corrected.
    pub pmc: bool,
}

pub enum Once {
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
    pub shape: Arc<ComplexShape>,
    /// Per-channel shim multipliers. `None` = single channel.
    /// Each entry is a per-sample shape; constant shims (from the official
    /// `rf_shims` extension) are stored as length-1 shapes, full pTx shapes
    /// (from the Martin pTx `shim_id` field) keep their per-sample resolution.
    pub shims: Option<Vec<Arc<ComplexShape>>>,
    pub rf_use: RfUse,
}

pub enum Gradient {
    Free {
        /// `[Hz/m]` - already FOV-scaled and rotated.
        amp: f64,
        /// `[s]`
        delay: f64,
        shape: Arc<Shape>,
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
    pub phase_shape: Option<Arc<Shape>>,
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

impl Sequence {
    pub fn from_seq(seq: &seq::Sequence, data: Data) -> Self {
        todo!()
    }
}
