use winnow::{
    ascii::till_line_ending,
    combinator::{alt, delimited, empty, opt, preceded, repeat, seq, terminated},
    prelude::*,
};

use super::{helpers::*, *};

pub fn file(input: &mut &str) -> ModalResult<Vec<Section>> {
    repeat(
        0..,
        preceded(
            opt(nl),
            alt((
                version.map(Section::Version),
                definitions.map(Section::Definitions),
                blocks.map(Section::Blocks),
                rfs.map(Section::Rfs),
                gradients.map(Section::Gradients),
                traps.map(Section::Traps),
                adcs.map(Section::Adcs),
                delays.map(Section::Delays),
                shapes.map(Section::Shapes),
            )),
        ),
    )
    .parse_next(input)
}

pub fn version(input: &mut &str) -> ModalResult<Version> {
    let major = delimited(tag_ws("major"), int, nl);
    let minor = delimited(tag_ws("minor"), int, nl);
    let revision = delimited(tag_ws("revision"), (int, opt(ident)), nl);

    (tag_nl("[VERSION]"), major, minor, revision)
        .map(|(_, major, minor, (revision, rev_suppl))| Version {
            major,
            minor,
            revision,
            rev_suppl,
        })
        .parse_next(input)
}

pub fn definitions(input: &mut &str) -> ModalResult<Vec<(String, String)>> {
    let def = (
        ident,
        ws,
        till_line_ending.map(|s: &str| s.trim().to_owned()),
        nl,
    )
        .map(|(key, _, value, _)| (key, value));
    preceded(tag_nl("[DEFINITIONS]"), repeat(0.., def)).parse_next(input)
}

pub fn blocks(input: &mut &str) -> ModalResult<Vec<Block>> {
    // TODO: maybe integrate ws into other parsers like int?
    let block = seq! { Block {
        id: int,
        dur: int.map(BlockDuration::DelayId),
        rf: int,
        gx: int,
        gy: int,
        gz: int,
        adc: int,
        ext: empty.value(0),
        _: nl,
    }};
    preceded(tag_nl("[BLOCKS]"), repeat(0.., block)).parse_next(input)
}

pub fn rfs(input: &mut &str) -> ModalResult<Vec<Rf>> {
    let rf = seq! {Rf {
        id: int,
        amp: float,
        mag_id: int,
        phase_id: int,
        time_id: empty.value(0),
        delay: int.map(|d: u32| d as f64 * 1e-6),
        freq: float,
        phase: float,
        // Shim indices of 0, 0 are treated as no shim - 0 is an invalid shape_id
        shim_id: opt((int, int)).map(|s| match s {
            Some((0, 0)) => None,
            _ => s,
        }),
        _: nl,
    }};
    preceded(tag_nl("[RF]"), repeat(0.., rf)).parse_next(input)
}

pub fn gradients(input: &mut &str) -> ModalResult<Vec<Gradient>> {
    let grad = || {
        seq! {Gradient {
            id: int,
            amp: float,
            shape_id: int,
            time_id: empty.value(0),
            delay: int.map(|d: u32| d as f64 * 1e-6),
            _: nl,
        }}
    };
    preceded(tag_nl("[GRADIENTS]"), repeat(0.., grad())).parse_next(input)
}

pub fn traps(input: &mut &str) -> ModalResult<Vec<Trap>> {
    let trap = seq! {Trap {
        id: int,
        amp: float,
        rise: int.map(|d: u32| d as f64 * 1e-6),
        flat: int.map(|d: u32| d as f64 * 1e-6),
        fall: int.map(|d: u32| d as f64 * 1e-6),
        delay: int.map(|d: u32| d as f64 * 1e-6),
        _: nl,
    }};
    preceded(tag_nl("[TRAP]"), repeat(0.., trap)).parse_next(input)
}

pub fn adcs(input: &mut &str) -> ModalResult<Vec<Adc>> {
    let adc = seq! {Adc {
        id: int,
        num: int,
        dwell: float.map(|d: f64| d * 1e-9),
        delay: int.map(|d: u32| d as f64 * 1e-6),
        freq: float,
        phase: float,
        _: nl,
    }};
    preceded(tag_nl("[ADC]"), repeat(0.., adc)).parse_next(input)
}

pub fn delays(input: &mut &str) -> ModalResult<Vec<Delay>> {
    let delay = seq! {Delay {
        id: int,
        delay: float.map(|d: f64| d * 1e-6),
        _: nl,
    }};
    preceded(tag_nl("[DELAYS]"), repeat(0.., delay)).parse_next(input)
}

pub fn raw_shape(input: &mut &str) -> ModalResult<(u32, (u32, Vec<f64>))> {
    // The spec and the exporter use different tags, we allow both.
    let shape_id = || delimited(alt((tag_ws("Shape_ID"), tag_ws("shape_id"))), int, nl);
    let num_samples = || {
        delimited(
            alt((tag_ws("Num_Uncompressed"), tag_ws("num_samples"))),
            int,
            nl,
        )
    };
    let samples = || repeat(0.., terminated(float, nl));

    seq!((shape_id(), (num_samples(), samples()))).parse_next(input)
}

pub fn shapes(input: &mut &str) -> ModalResult<Vec<Shape>> {
    // The spec says optional RLE only since version 1.4 but seems to be used earlier.
    // We allow it in all versions unless there is a bug report for failing decompression.
    let shape = raw_shape.try_map(|(id, (num_samples, samples))| {
        if samples.len() == num_samples as usize {
            Ok(Shape { id, samples })
        } else {
            decompress_shape(samples, num_samples).map(|samples| Shape { id, samples })
        }
    });
    preceded(tag_nl("[SHAPES]"), repeat(0.., shape)).parse_next(input)
}
