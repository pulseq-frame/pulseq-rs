use ezpc::*;

use super::{helpers::*, *};

pub fn file() -> Parser<impl Parse<Output = Vec<Section>>> {
    nl().opt()
        + (version().map(Section::Version)
            | definitions().map(Section::Definitions)
            | blocks().map(Section::Blocks)
            | rfs().map(Section::Rfs)
            | gradients().map(Section::Gradients)
            | traps().map(Section::Traps)
            | adcs().map(Section::Adcs)
            | delays().map(Section::Delays)
            | shapes().map(Section::Shapes))
        .repeat(0..)
}

pub fn version() -> Parser<impl Parse<Output = Version>> {
    let major = tag_ws("major") + int() + nl();
    let minor = tag_ws("minor") + int() + nl();
    let revision = tag_ws("revision") + int() + ident().opt() + nl();

    (tag_nl("[VERSION]") + major + minor + revision).map(
        |((major, minor), (revision, rev_suppl))| Version {
            major,
            minor,
            revision,
            rev_suppl,
        },
    )
}

pub fn definitions() -> Parser<impl Parse<Output = Definitions>> {
    raw_definitions().convert(parse_defs, "Failed to parse definitions")
}

pub fn raw_definitions() -> Parser<impl Parse<Output = Vec<(String, String)>>> {
    let def = ident() + ws() + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + nl();
    tag_nl("[DEFINITIONS]") + def.repeat(1..)
}

pub fn parse_defs(defs: Vec<(String, String)>) -> Result<Definitions, ParseError> {
    let mut defs: HashMap<_, _> = defs.into_iter().collect();

    // Before pulseq 1.4, defining raster times was not mandatory. This is a
    // flaw in the specification, because without the raster time, the duration
    // of RF pulses and non-trap gradients is completely undefined. The
    // official Siemens interpreter uses default values for missing raster
    // times, which can be seen as the ground truth even if not given by the
    // specification.

    // TODO: Remove duplication with TimeRaster default impl

    Ok(Definitions {
        grad_raster: defs
            .remove("GradientRasterTime")
            .map(|s| s.parse())
            .unwrap_or(Ok(10e-6))?,
        rf_raster: defs
            .remove("RadiofrequencyRasterTime")
            .map(|s| s.parse())
            .unwrap_or(Ok(1e-6))?,
        adc_raster: defs
            .remove("AdcRasterTime")
            .map(|s| s.parse())
            .unwrap_or(Ok(0.1e-6))?,
        block_dur_raster: defs
            .remove("BlockDurationRaster")
            .map(|s| s.parse())
            .unwrap_or(Ok(10e-6))?,
        name: defs.remove("Name"),
        fov: defs.remove("FOV").map(parse_fov).transpose()?,
        rest: defs,
    })
}

pub fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (ws().opt() + int() + (ws() + int()).repeat(6)).map(|(id, tags)| Block {
        id,
        dur: BlockDuration::DelayId(tags[0]),
        rf: tags[1],
        gx: tags[2],
        gy: tags[3],
        gz: tags[4],
        adc: tags[5],
        ext: 0, // Modified: 1.2 doesn't have extensions
    });
    tag_nl("[BLOCKS]") + (block + nl()).repeat(1..)
}

pub fn rfs() -> Parser<impl Parse<Output = Vec<Rf>>> {
    // same as 1.3
    let i = || ws() + int();
    let f = || ws() + float();
    let rf = (ws().opt() + int() + f() + i() + i() + i() + f() + f()).map(
        |((((((id, amp), mag_id), phase_id), delay), freq), phase)| Rf {
            id,
            amp,
            mag_id,
            phase_id,
            time_id: 0,
            delay: delay as f32 * 1e-6,
            freq,
            phase,
        },
    );
    tag_nl("[RF]") + (rf + nl()).repeat(1..)
}

pub fn gradients() -> Parser<impl Parse<Output = Vec<Gradient>>> {
    let i = || ws() + int();
    let f = ws() + float();
    let grad =
        (ws().opt() + int() + f + i() + i()).map(|(((id, amp), shape_id), delay)| Gradient {
            id,
            amp,
            shape_id,
            time_id: 0,
            delay: delay as f32 * 1e-6,
        });
    tag_nl("[GRADIENTS]") + (grad + nl()).repeat(1..)
}

pub fn traps() -> Parser<impl Parse<Output = Vec<Trap>>> {
    let i = || ws() + int();
    let f = ws() + float();
    let trap = (ws().opt() + int() + f + i() + i() + i() + i()).map(
        |(((((id, amp), rise), flat), fall), delay)| Trap {
            id,
            amp,
            rise: rise as f32 * 1e-6,
            flat: flat as f32 * 1e-6,
            fall: fall as f32 * 1e-6,
            delay: delay as f32 * 1e-6,
        },
    );
    tag_nl("[TRAP]") + (trap + nl()).repeat(1..)
}

pub fn adcs() -> Parser<impl Parse<Output = Vec<Adc>>> {
    let i = || ws() + int();
    let f = || ws() + float();
    let adc = (ws().opt() + int() + i() + f() + i() + f() + f()).map(
        |(((((id, num), dwell), delay), freq), phase)| Adc {
            id,
            num,
            dwell: dwell * 1e-9,
            delay: delay as f32 * 1e-6,
            freq,
            phase,
        },
    );
    tag_nl("[ADC]") + (adc + nl()).repeat(1..)
}

pub fn delays() -> Parser<impl Parse<Output = Vec<Delay>>> {
    let delay = (ws().opt() + int() + ws() + float()).map(|(id, delay)| Delay {
        id,
        delay: delay * 1e-6,
    });
    tag_nl("[DELAYS]") + (delay + nl()).repeat(1..)
}

pub fn raw_shape() -> Parser<impl Parse<Output = (u32, (u32, Vec<f32>))>> {
    // The spec and the exporter use different tags, we allow both.
    let shape_id = (tag_ws("Shape_ID") | tag_ws("shape_id")) + int() + nl();
    let num_samples = (tag_ws("Num_Uncompressed") | tag_ws("num_samples")) + int() + nl();
    let samples = num_samples + (ws().opt() + float() + nl()).repeat(1..);
    shape_id + samples
}

pub fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    let shape = raw_shape().convert(
        |(id, (num_samples, samples))| {
            decompress_shape(samples, num_samples).map(|samples| Shape { id, samples })
        },
        "Failed to decompress shape",
    );
    tag_nl("[SHAPES]") + shape.repeat(1..)
}
