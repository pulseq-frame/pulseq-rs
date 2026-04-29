use winnow::{
    ascii::till_line_ending,
    combinator::{alt, cut_err, delimited, empty, opt, preceded, repeat, seq, terminated},
    error::StrContext,
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
    seq! { Version {
        _: tag_nl("[VERSION]"),
        major: cut_err(delimited(tag_ws("major"), int, nl)),
        minor: cut_err(delimited(tag_ws("minor"), int, nl)),
        revision: cut_err(preceded(tag_ws("revision"), int)),
        rev_suppl: cut_err(terminated(opt(ident), nl))
    }}
    .context(StrContext::Label("[VERSION] section"))
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

    preceded(tag_nl("[DEFINITIONS]"), repeat(0.., def))
        .context(StrContext::Label("[DEFINITIONS] section"))
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
        ext: empty.value(0),
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
        time_id: empty.value(0),
        center: empty.value(None),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        freq_rel: empty.value(1.0),
        phase_rel: empty.value(1.0),
        freq_off: cut_err(float),
        phase_off: cut_err(float),
        shim_id: opt((int, int)).map(|s| match s {
            Some((0, 0)) => None,  // no shim - 0 is an invalid shape_id
            _ => s,
        }),
        rf_use: empty.value('u'), // undefined use
        _: cut_err(nl),
    }}
    .context(StrContext::Label("rf record"));

    preceded(tag_nl("[RF]"), repeat(0.., rf))
        .context(StrContext::Label("[RF] section"))
        .parse_next(input)
}

pub fn gradients(input: &mut &str) -> ModalResult<Vec<Gradient>> {
    let grad = || {
        seq! {Gradient {
            id: int,
            amp: cut_err(float),
            shape_id: cut_err(int),
            time_id: empty.value(0),
            delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
            _: cut_err(nl),
        }}
        .context(StrContext::Label("gradient record"))
    };

    preceded(tag_nl("[GRADIENTS]"), repeat(0.., grad()))
        .context(StrContext::Label("[GRADIENTS] section"))
        .parse_next(input)
}

pub fn traps(input: &mut &str) -> ModalResult<Vec<Trap>> {
    let trap = seq! {Trap {
        id: int,
        amp: cut_err(float),
        rise: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        flat: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        fall: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("trap record"));

    preceded(tag_nl("[TRAP]"), repeat(0.., trap))
        .context(StrContext::Label("[TRAP] section"))
        .parse_next(input)
}

pub fn adcs(input: &mut &str) -> ModalResult<Vec<Adc>> {
    let adc = seq! {Adc {
        id: int,
        num: cut_err(int),
        dwell: cut_err(float).map(|d: f64| d * 1e-9),
        delay: cut_err(int).map(|d: u32| d as f64 * 1e-6),
        freq: cut_err(float),
        phase: cut_err(float),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("adc record"));

    preceded(tag_nl("[ADC]"), repeat(0.., adc))
        .context(StrContext::Label("[ADC] section"))
        .parse_next(input)
}

pub fn delays(input: &mut &str) -> ModalResult<Vec<Delay>> {
    let delay = seq! {Delay {
        id: int,
        delay: cut_err(float).map(|d: f64| d * 1e-6),
        _: cut_err(nl),
    }}
    .context(StrContext::Label("delay record"));

    preceded(tag_nl("[DELAYS]"), repeat(0.., delay))
        .context(StrContext::Label("[DELAYS] section"))
        .parse_next(input)
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

    seq!((shape_id(), cut_err((num_samples(), samples()))))
        .context(StrContext::Label("shape"))
        .parse_next(input)
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

    preceded(tag_nl("[SHAPES]"), repeat(0.., shape))
        .context(StrContext::Label("[SHAPES] section"))
        .parse_next(input)
}
