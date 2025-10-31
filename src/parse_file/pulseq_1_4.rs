use ezpc::*;

use super::pulseq_1_2::{adcs, definitions, shapes, traps, version};
use super::pulseq_1_3::extensions;
use super::{helpers::*, *};

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

pub fn signature() -> Parser<impl Parse<Output = Signature>> {
    let typ = tag_ws("Type")
        + is_a(char::is_alphanumeric)
            .repeat(1..)
            .map(|s| s.to_owned())
        + nl();
    let hash = tag_ws("Hash") + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + nl();

    (tag_nl("[SIGNATURE]") + typ + hash).map(|(typ, hash)| Signature { typ, hash })
}

pub fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
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
    tag_nl("[BLOCKS]") + (block + nl()).repeat(0..)
}

pub fn rfs() -> Parser<impl Parse<Output = Vec<Rf>>> {
    let i = || ws() + int();
    let f = || ws() + float();
    let rf = (ws().opt() + int() + f() + i() + i() + i() + i() + f() + f() + (i() + i()).opt())
        .map(
            |((((((((id, amp), mag_id), phase_id), time_id), delay), freq), phase), shim_id_raw)| {
                // Shim indices of 0, 0 are treated as no shim - 0 is an invalid shape_id
                let shim_id = match shim_id_raw {
                    Some((0, 0)) => None,
                    _ => shim_id_raw,
                };
                Rf {
                    id,
                    amp,
                    mag_id,
                    phase_id,
                    time_id,
                    delay: delay as f64 * 1e-6,
                    freq,
                    phase,
                    shim_id,
                }
            },
        );
    tag_nl("[RF]") + (rf + nl()).repeat(0..)
}

pub fn gradients() -> Parser<impl Parse<Output = Vec<Gradient>>> {
    let i = || ws() + int();
    let f = ws() + float();
    let grad = (ws().opt() + int() + f + i() + i() + i()).map(
        |((((id, amp), shape_id), time_id), delay)| Gradient {
            id,
            amp,
            shape_id,
            time_id,
            delay: delay as f64 * 1e-6,
        },
    );
    tag_nl("[GRADIENTS]") + (grad + nl()).repeat(0..)
}
