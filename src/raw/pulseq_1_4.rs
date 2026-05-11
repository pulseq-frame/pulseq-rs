use winnow::ascii::{alphanumeric1, till_line_ending};
use winnow::combinator::{alt, cut_err, delimited, empty, opt, preceded, repeat, seq};
use winnow::error::StrContext;
use winnow::prelude::*;

use super::pulseq_1_2::{adcs, definitions, shapes, traps, version};
use super::pulseq_1_3::{extension_refs, extension_specs};
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
                alt((
                    extension_refs.map(Section::ExtensionRefs),
                    extension_specs.map(Section::ExtensionSpecs),
                    shapes.map(Section::Shapes),
                    signature.map(Section::Signature),
                )),
            )),
        ),
    )
    .parse_next(input)
}

pub fn signature(input: &mut &str) -> ModalResult<Signature> {
    let mut typ = cut_err(delimited(
        tag_ws("Type"),
        alphanumeric1.map(|s: &str| s.to_owned()),
        nl,
    ));
    let mut hash = cut_err(delimited(
        tag_ws("Hash"),
        till_line_ending.map(|s: &str| s.trim().to_owned()),
        nl,
    ));

    seq! { Signature {
        _: tag_nl("[SIGNATURE]"),
        typ: typ,
        hash: hash,
    }}
    .context(StrContext::Label("[SIGNATURE] section"))
    .parse_next(input)
}

pub fn blocks(input: &mut &str) -> ModalResult<Vec<Block>> {
    let block = seq! { Block {
        id: int,
        dur: cut_err(int).map(BlockDuration::Duration),
        rf: cut_err(int),
        gx: cut_err(int),
        gy: cut_err(int),
        gz: cut_err(int),
        adc: cut_err(int),
        ext: cut_err(int),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("block record"));

    preceded(tag_nl("[BLOCKS]"), repeat(0.., block))
        .context(StrContext::Label("[BLOCKS] section"))
        .parse_next(input)
}

pub fn rfs(input: &mut &str) -> ModalResult<Vec<Rf>> {
    let rf = seq! {Rf {
        id: int,
        amp: cut_err(float),
        mag_id: cut_err(int),
        phase_id: cut_err(int),
        time_id: cut_err(signed_int),
        center: empty.value(None),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        freq_rel: empty.value(1.0),
        phase_rel: empty.value(1.0),
        freq_off: cut_err(float),
        phase_off: cut_err(float),
        // Shim indices of 0, 0 are treated as no shim - 0 is an invalid shape_id
        shim_id: opt((int, int)).map(|s| match s {
            Some((0, 0)) => None,
            _ => s,
        }),
        rf_use: empty.value(RfUse::Undefined),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("rf record"));

    preceded(tag_nl("[RF]"), repeat(0.., rf))
        .context(StrContext::Label("[RF] section"))
        .parse_next(input)
}

pub fn gradients(input: &mut &str) -> ModalResult<Vec<Gradient>> {
    let grad = seq! {Gradient {
        id: int,
        amp: cut_err(float),
        first: empty.value(None),
        last: empty.value(None),
        shape_id: cut_err(int),
        time_id: cut_err(signed_int),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("gradient record"));

    preceded(tag_nl("[GRADIENTS]"), repeat(0.., grad))
        .context(StrContext::Label("[GRADIENTS] section"))
        .parse_next(input)
}
