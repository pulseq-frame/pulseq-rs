use winnow::ascii::till_line_ending;
use winnow::combinator::{alt, cut_err, opt, preceded, repeat, seq};
use winnow::error::StrContext;
use winnow::prelude::*;

use super::pulseq_1_2::{adcs, definitions, delays, gradients, rfs, shapes, traps, version};
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
                alt((
                    extension_refs.map(Section::ExtensionRefs),
                    extension_specs.map(Section::ExtensionSpecs),
                    shapes.map(Section::Shapes),
                )),
            )),
        ),
    )
    .parse_next(input)
}

pub fn blocks(input: &mut &str) -> ModalResult<Vec<Block>> {
    let block = seq! { Block {
        id: int,
        dur: cut_err(int).map(BlockDuration::DelayId),
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

// [EXTENSIONS] section format:
//
// `ExtensionRef` defined by 4 numbers: "<id> <type> <ref> <next>"
// (introduced by the `[EXTENSIONS]` header).
//
// Followed by extension specifications. Each `ExtensionSpec` starts with
// `extension <STRING_ID> <type>` and is followed by a list of `ExtensionObject`
// instances - one line, `<id>` + extension specific data.
//
// In the spec all refs of one [EXTENSIONS] block come before all specs of that
// block, but we don't enforce that here: refs are parsed by `extension_refs`
// (anchored on the [EXTENSIONS] header), and each `extension <NAME> <ID>` block
// is parsed independently by `extension_specs`. The downstream conversion
// merges them via the SectionData pipeline, so interleaved or repeated blocks
// just concatenate.

pub fn extension_refs(input: &mut &str) -> ModalResult<Vec<ExtensionRef>> {
    let ext_ref = seq! { ExtensionRef {
        id: int,
        spec_id: cut_err(int),
        obj_id: cut_err(int),
        next: cut_err(int),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("extension reference"));

    preceded(tag_nl("[EXTENSIONS]"), repeat(0.., ext_ref))
        .context(StrContext::Label("[EXTENSIONS] section"))
        .parse_next(input)
}

pub fn extension_specs(input: &mut &str) -> ModalResult<Vec<ExtensionSpec>> {
    let ext_obj = || {
        seq! { ExtensionObject {
            id: int,
            data: till_line_ending.map(|s: &str| s.trim().to_owned()),
            _: cut_err(nl),
        }}
        .context(StrContext::Label("extension object"))
    };

    let ext_spec = seq! { ExtensionSpec {
        _: tag_ws("extension"),
        name: cut_err(ident),
        id: cut_err(int),
        _: cut_err(nl),
        instances: cut_err(repeat(1.., ext_obj())),
    }}
    .context(StrContext::Label("extension specification"));

    repeat(1.., ext_spec)
        .context(StrContext::Label("extension specifications"))
        .parse_next(input)
}
