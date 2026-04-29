use winnow::combinator::{alt, cut_err, empty, opt, preceded, repeat, seq};
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::token::one_of;

use super::pulseq_1_2::{adcs, definitions, shapes, traps, version};
use super::pulseq_1_3::extensions;
use super::pulseq_1_4::{blocks, gradients, signature};
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

pub fn rfs(input: &mut &str) -> ModalResult<Vec<Rf>> {
    let rf = seq! {Rf {
        id: int,
        amp: cut_err(float),
        mag_id: cut_err(int),
        phase_id: cut_err(int),
        time_id: cut_err(int),
        center: cut_err(float).map(|x| Some(x * 1e-6)),
        delay: cut_err(int).map(|x: u32| x as f64 * 1e-6),
        freq_rel: cut_err(float).map(|x| x * 1e-6),
        phase_rel: cut_err(float).map(|x| x * 1e-6),
        freq_off: cut_err(float),
        phase_off: cut_err(float),
        // pulseq 1.5 uses the shim extension instead of Martins modification
        shim_id: empty.value(None),
        rf_use: cut_err(preceded(ws, one_of(['e', 'r', 'i', 's', 'p', 'o', 'u']))),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("rf record"));

    preceded(tag_nl("[RF]"), repeat(0.., rf))
        .context(StrContext::Label("[RF] section"))
        .parse_next(input)
}
