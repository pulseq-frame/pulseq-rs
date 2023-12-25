use ezpc::*;

use super::common::*;
use super::*;
use pulseq_all::*;

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
            | extensions().map(Section::Extensions)
            | shapes().map(Section::Shapes))
        .repeat(0..)
}

fn version() -> Parser<impl Parse<Output = Version>> {
    raw_version().try_map(|v| {
        if v.major == 1 && v.minor == 3 {
            Ok(v)
        } else {
            Err(ParseError::Generic)
        }
    })
}

fn definitions() -> Parser<impl Parse<Output = Definitions>> {
    raw_definitions().map(|def_vec| Definitions::V131(def_vec.into_iter().collect()))
}

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (ws().opt() + int() + (ws() + int()).repeat(7)).map(|(id, tags)| Block::V131 {
        id,
        delay: tags[0],
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
        delay: delay as f32 * 1e-6,
    });
    tag_nl("[DELAYS]") + (delay + nl()).repeat(1..)
}

fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    let shape = raw_shape().try_map(|(id, (num_samples, samples))| {
        decompress_shape(samples, num_samples).map(|samples| Shape { id, samples })
    });
    tag_nl("[SHAPES]") + shape.repeat(1..)
}
