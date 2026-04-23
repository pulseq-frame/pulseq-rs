use winnow::ascii::till_line_ending;
use winnow::combinator::{alt, opt, preceded, repeat, seq};
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
                    extensions.map(Section::Extensions),
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
        dur: int.map(BlockDuration::DelayId),
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

pub fn extensions(input: &mut &str) -> ModalResult<Extensions> {
    // [EXTENSIONS] section is a table where each line is a entry
    // `ExtensionRef` defined by 4 numbers: "<id> <type> <ref> <next>""
    //
    // it is followed by the specification of the extensions
    // `ExtensionSpec` is defined by "extension <STRING_ID> <type>"
    //
    // each extension (labels, triggers etc) is followed by a list of instances
    // `ExtensionObject` is a single line, <id> + extension specific data

    let ext_ref = || {
        seq! { ExtensionRef {
            id: int,
            spec_id: int,
            obj_id: int,
            next: int,
            _: nl,
        }}
    };
    let ext_obj = || {
        seq! { ExtensionObject {
            id: int,
            data: till_line_ending.map(|s: &str| s.trim().to_owned()),
            _: nl,
        }}
    };
    let ext_spec = move || {
        seq! { ExtensionSpec {
            _: tag_ws("extension"),
            name: ident,
            id: int,
            _: nl,
            instances: repeat(1.., ext_obj())
        }}
    };

    seq! { Extensions {
        _: tag_nl("[EXTENSIONS]"),
        refs: repeat(0.., ext_ref()),
        specs: repeat(0.., ext_spec())
    }}
    .parse_next(input)
}
