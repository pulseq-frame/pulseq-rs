use winnow::ascii::{alphanumeric1, till_line_ending};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, seq};
use winnow::prelude::*;

use super::pulseq_1_2::{adcs, definitions, shapes, traps, version};
use super::pulseq_1_3::extensions;
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
                extensions.map(Section::Extensions),
                alt((
                    shapes.map(Section::Shapes),
                    signature.map(Section::Signature),
                )),
            )),
        ),
    )
    .parse_next(input)
}

pub fn signature(input: &mut &str) -> ModalResult<Signature> {
    let mut typ = delimited(
        tag_ws("Type"),
        alphanumeric1.map(|s: &str| s.to_owned()),
        nl,
    );
    let mut hash = delimited(
        tag_ws("Hash"),
        till_line_ending.map(|s: &str| s.trim().to_owned()),
        nl,
    );

    seq! { Signature {
        _: tag_nl("[SIGNATURE]"),
        typ: typ,
        hash: hash
    }}
    .parse_next(input)
}

pub fn blocks(input: &mut &str) -> ModalResult<Vec<Block>> {
    let block = seq! { Block {
        _: opt(ws),
        id: int,
        dur: preceded(ws, int).map(BlockDuration::Duration),
        rf: preceded(ws, int),
        gx: preceded(ws, int),
        gy: preceded(ws, int),
        gz: preceded(ws, int),
        adc: preceded(ws, int),
        ext: preceded(ws, int),
        _: nl,
    }};
    preceded(tag_nl("[BLOCKS]"), repeat(0.., block)).parse_next(input)
}

pub fn rfs(input: &mut &str) -> ModalResult<Vec<Rf>> {
    let i = || preceded(ws, int);
    let f = || preceded(ws, float);

    let rf = seq! {Rf {
        _: opt(ws),
        id: int,
        amp: f(),
        mag_id: i(),
        phase_id: i(),
        time_id: i(),
        delay: i().map(|d: u32| d as f64 * 1e-6),
        freq: f(),
        phase: f(),
        // Shim indices of 0, 0 are treated as no shim - 0 is an invalid shape_id
        shim_id: opt((i(), i())).map(|s| match s {
            Some((0, 0)) => None,
            _ => s,
        }),
        _: nl,
    }};
    preceded(tag_nl("[RF]"), repeat(0.., rf)).parse_next(input)
}

pub fn gradients(input: &mut &str) -> ModalResult<Vec<Gradient>> {
    let i = || preceded(ws, int);
    let f = || preceded(ws, float);

    let grad = seq! {Gradient {
        _: opt(ws),
        id: int,
        amp: f(),
        shape_id: i(),
        time_id: i(),
        delay: i().map(|d: u32| d as f64 * 1e-6),
        _: nl,
    }};
    preceded(tag_nl("[GRADIENTS]"), repeat(0.., grad)).parse_next(input)
}
