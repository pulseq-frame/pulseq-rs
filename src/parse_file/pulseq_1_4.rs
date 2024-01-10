use ezpc::*;

use super::{helpers::*, *};
use common::*;

pub fn file() -> Parser<impl Parse<Output = Vec<Section>>> {
    nl().opt()
        + (version().map(Section::Version)
            | signature().map(Section::Signature)
            | definitions().map(Section::Definitions)
            | blocks().map(Section::Blocks)
            | rfs().map(Section::Rfs)
            | gradients().map(Section::Gradients)
            | traps().map(Section::Traps)
            | adcs().map(Section::Adcs)
            | extensions().map(Section::Extensions)
            | shapes().map(Section::Shapes))
        .repeat(0..)
}

fn signature() -> Parser<impl Parse<Output = Signature>> {
    let typ = tag_ws("Type")
        + is_a(char::is_alphanumeric)
            .repeat(1..)
            .map(|s| s.to_owned())
        + nl();
    let hash = tag_ws("Hash") + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + nl();

    (tag_nl("[SIGNATURE]") + typ + hash).map(|(typ, hash)| Signature { typ, hash })
}

fn definitions() -> Parser<impl Parse<Output = Definitions>> {
    raw_definitions().convert(parse_defs, "Failed to parse definitions")
}

fn parse_defs(defs: Vec<(String, String)>) -> Result<Definitions, ParseError> {
    let mut defs: HashMap<_, _> = defs.into_iter().collect();

    Ok(Definitions {
        grad_raster: defs
            .remove("GradientRasterTime")
            .ok_or(ParseError::Generic)?
            .parse()?,
        rf_raster: defs
            .remove("RadiofrequencyRasterTime")
            .ok_or(ParseError::Generic)?
            .parse()?,
        adc_raster: defs
            .remove("AdcRasterTime")
            .ok_or(ParseError::Generic)?
            .parse()?,
        block_dur_raster: defs
            .remove("BlockDurationRaster")
            .ok_or(ParseError::Generic)?
            .parse()?,
        name: defs.remove("Name"),
        fov: defs.remove("FOV").map(parse_fov).transpose()?,
        rest: defs,
    })
}

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (ws().opt() + int() + (ws() + int()).repeat(7)).map(|(id, tags)| Block {
        id,
        dur: BlockDuration::Duration(tags[0]),
        rf: tags[1],
        gx: tags[2],
        gy: tags[3],
        gz: tags[4],
        adc: tags[5],
        ext: tags[6],
    });
    tag_nl("[BLOCKS]") + (block + nl()).repeat(1..)
}

fn rfs() -> Parser<impl Parse<Output = Vec<Rf>>> {
    let i = || ws() + int();
    let f = || ws() + float();
    let rf = (ws().opt() + int() + f() + i() + i() + i() + i() + f() + f()).map(
        |(((((((id, amp), mag_id), phase_id), time_id), delay), freq), phase)| Rf {
            id,
            amp,
            mag_id,
            phase_id,
            time_id,
            delay: delay as f32 * 1e-6,
            freq,
            phase,
        },
    );
    tag_nl("[RF]") + (rf + nl()).repeat(1..)
}

fn gradients() -> Parser<impl Parse<Output = Vec<Gradient>>> {
    let i = || ws() + int();
    let f = ws() + float();
    let grad = (ws().opt() + int() + f + i() + i() + i()).map(
        |((((id, amp), shape_id), time_id), delay)| Gradient {
            id,
            amp,
            shape_id,
            time_id,
            delay: delay as f32 * 1e-6,
        },
    );
    tag_nl("[GRADIENTS]") + (grad + nl()).repeat(1..)
}

fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    let shape = raw_shape().convert(
        |(id, (num_samples, samples))| {
            if samples.len() == num_samples as usize {
                Ok(Shape { id, samples })
            } else {
                decompress_shape(samples, num_samples).map(|samples| Shape { id, samples })
            }
        },
        "Failed to decompress shape",
    );
    tag_nl("[SHAPES]") + shape.repeat(1..)
}