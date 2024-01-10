use ezpc::*;

use super::pulseq_1_2::{adcs, definitions, delays, gradients, rfs, shapes, traps, version};
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
            | extensions().map(Section::Extensions)
            | shapes().map(Section::Shapes))
        .repeat(0..)
}

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (ws().opt() + int() + (ws() + int()).repeat(7)).map(|(id, tags)| Block {
        id,
        dur: BlockDuration::DelayId(tags[0]),
        rf: tags[1],
        gx: tags[2],
        gy: tags[3],
        gz: tags[4],
        adc: tags[5],
        ext: tags[6],
    });
    tag_nl("[BLOCKS]") + (block + nl()).repeat(1..)
}

pub fn extensions() -> Parser<impl Parse<Output = Extensions>> {
    let rest_of_line = none_of("\n").repeat(1..).map(|s| s.trim().to_owned());
    let i = || ws() + int();
    let ext_ref =
        (ws().opt() + int() + i() + i() + i() + nl()).map(|(((id, spec_id), obj_id), next)| {
            ExtensionRef {
                id,
                spec_id,
                obj_id,
                next,
            }
        });
    let ext_obj =
        (ws().opt() + int() + rest_of_line + nl()).map(|(id, data)| ExtensionObject { id, data });
    let ext_spec = (tag_ws("extension") + ident() + ws() + int() + nl() + ext_obj.repeat(1..)).map(
        |((name, id), instances)| ExtensionSpec {
            id,
            name,
            instances,
        },
    );
    (tag_nl("[EXTENSIONS]") + ext_ref.repeat(1..) + ext_spec.repeat(1..))
        .map(|(refs, specs)| Extensions { refs, specs })
}
