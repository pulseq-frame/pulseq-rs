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
        id: int,
        dur: int.map(BlockDuration::Duration),
        rf: int,
        gx: int,
        gy: int,
        gz: int,
        adc: int,
        ext: int,
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
        time_id: int,
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
    let grad = seq! {Gradient {
        id: int,
        amp: float,
        shape_id: int,
        time_id: int,
        delay: int.map(|d: u32| d as f64 * 1e-6),
        _: nl,
    }};
    preceded(tag_nl("[GRADIENTS]"), repeat(0.., grad)).parse_next(input)
}
