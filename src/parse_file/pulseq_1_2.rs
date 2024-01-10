use ezpc::*;

use super::{helpers::*, *};
use common::*;

// TODO: would be cleaner to build the different file versions up on each other:
// start with a full 1.2 parser (no common files), then in the 1.3 parser re-export
// all unchanged parsers and only re-define what changed, 1.4 do the same based on 1.3

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
            // Modified: no extensions
            | shapes().map(Section::Shapes))
        .repeat(0..)
}

fn definitions() -> Parser<impl Parse<Output = Definitions>> {
    // not modified compared to 1.3
    raw_definitions().convert(parse_defs, "Failed to parse definitions")
}

fn parse_defs(defs: Vec<(String, String)>) -> Result<Definitions, ParseError> {
    let mut defs: HashMap<_, _> = defs.into_iter().collect();

    // Before pulseq 1.4, defining raster times was not mandatory. This is a
    // flaw in the specification, because without the raster time, the duration
    // of RF pulses and non-trap gradients is completely undefined. The
    // official Siemens interpreter uses default values for missing raster
    // times, which can be seen as the ground truth even if not given by the
    // specification.

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

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
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

fn rfs() -> Parser<impl Parse<Output = Vec<Rf>>> {
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

fn gradients() -> Parser<impl Parse<Output = Vec<Gradient>>> {
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

fn delays() -> Parser<impl Parse<Output = Vec<Delay>>> {
    let delay = (ws().opt() + int() + ws() + float()).map(|(id, delay)| Delay {
        id,
        delay: delay * 1e-6,
    });
    tag_nl("[DELAYS]") + (delay + nl()).repeat(1..)
}

fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    let shape = raw_shape().convert(
        |(id, (num_samples, samples))| {
            decompress_shape(samples, num_samples).map(|samples| Shape { id, samples })
        },
        "Failed to decompress shape",
    );
    tag_nl("[SHAPES]") + shape.repeat(1..)
}