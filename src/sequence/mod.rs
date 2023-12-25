// This module describes a pulseq sequence, boiled down to the necessary info.

use std::rc::Rc;

pub struct Sequence {
    meta: Metadata,
    blocks: Vec<Block>,
}

pub struct Metadata {
    name: Option<String>,
    fov: Option<(f32, f32, f32)>,
}

pub struct Block {
    duration: f32,
    rf: Option<Rc<Rf>>,
    gx: Option<Rc<Gradient>>,
    gy: Option<Rc<Gradient>>,
    gz: Option<Rc<Gradient>>,
    adc: Option<Rc<Adc>>,
}

pub struct Rf {
    /// Unit: `[Hz]`
    amp: f32,
    /// Unit: `[rad]`
    phase: f32,
    amp_shape: Rc<Shape>,
    phase_shape: Rc<Shape>,
    /// Unit: `[s]`
    delay: f32,
    /// Unit: `[Hz]`
    freq: f32,
}

pub enum Gradient {
    Free {
        /// Unit: `[Hz/m]`
        amp: f32,
        shape: Rc<Shape>,
        time: Rc<Shape>,
        /// Unit: `[s]`
        delay: f32,
    },
    Trap {
        /// Unit: `[Hz/m]`
        amp: f32,
        /// Unit: `[s]`
        rise: f32,
        /// Unit: `[s]`
        flat: f32,
        /// Unit: `[s]`
        fall: f32,
        /// Unit: `[s]`
        delay: f32,
    }
}

pub struct Adc {
    num: u32,
    /// Unit: `[s]`
    dwell: f32,
    /// Unit: `[s]`
    delay: f32,
    /// Unit: `[Hz]`
    freq: f32,
    /// Unit: `[rad]`
    phase: f32,
}

pub struct Shape(Vec<f32>);