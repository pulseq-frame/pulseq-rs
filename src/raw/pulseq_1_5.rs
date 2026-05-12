use winnow::combinator::{alt, cut_err, empty, opt, preceded, repeat, seq};
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::token::one_of;

use super::pulseq_1_2::{definitions, shapes, traps, version};
use super::pulseq_1_3::{extension_refs, extension_specs};
use super::pulseq_1_4::{blocks, signature};
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

pub fn rfs(input: &mut &str) -> ModalResult<Vec<Rf>> {
    let rf = seq! {Rf {
        id: int,
        amp: cut_err(float),
        mag_id: cut_err(int),
        phase_id: cut_err(int),
        time_id: cut_err(signed_int),
        center: cut_err(float).map(|x| Some(x * 1e-6)),
        delay: cut_err(int).map(|x: u32| x as f64 * 1e-6),
        freq_rel: cut_err(float).map(|x| x * 1e-6),
        phase_rel: cut_err(float).map(|x| x * 1e-6),
        freq_off: cut_err(float),
        phase_off: cut_err(float),
        // pulseq 1.5 uses the shim extension instead of Martins modification
        shim_id: empty.value(None),
        rf_use: cut_err(preceded(ws, one_of(['e', 'r', 'i', 's', 'p', 'o', 'u']))).map(
            |c| match c {
                'e' => RfUse::Excitation,
                'r' => RfUse::Refocusing,
                'i' => RfUse::Inversion,
                's' => RfUse::Saturation,
                'p' => RfUse::Preparation,
                'o' => RfUse::Other,
                'u' => RfUse::Undefined,
                _ => unreachable!("one_of restricts to listed chars"),
            },
        ),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("rf record"));

    preceded(tag_nl("[RF]"), repeat(0.., rf))
        .context(StrContext::Label("[RF] section"))
        .parse_next(input)
}

pub fn adcs(input: &mut &str) -> ModalResult<Vec<Adc>> {
    let adc = seq! {Adc {
        id: int,
        num: cut_err(int),
        dwell: cut_err(float).map(|d: f64| d * 1e-9),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        freq_rel: cut_err(float).map(|x| x * 1e-6),
        phase_rel: cut_err(float).map(|x| x * 1e-6),
        freq_off: cut_err(float),
        phase_off: cut_err(float),
        phase_shape_id: cut_err(int),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("adc record"));

    preceded(tag_nl("[ADC]"), repeat(0.., adc))
        .context(StrContext::Label("[ADC] section"))
        .parse_next(input)
}

pub fn gradients(input: &mut &str) -> ModalResult<Vec<Gradient>> {
    let grad = seq! {Gradient {
        id: int,
        amp: cut_err(float),
        first: cut_err(float.map(Some)),
        last: cut_err(float.map(Some)),
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
