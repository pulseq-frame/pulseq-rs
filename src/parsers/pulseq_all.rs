use ezpc::*;

use super::common::*;
use super::*;

// Parsers for things that didn't change within the supported pulseq versions

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

pub fn raw_definitions() -> Parser<impl Parse<Output = Vec<(String, String)>>> {
    let def = ident() + ws() + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + nl();
    tag_nl("[DEFINITIONS]") + def.repeat(1..)
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

pub fn raw_shape() -> Parser<impl Parse<Output = (u32, (u32, Vec<f32>))>> {
    // The spec and the exporter use different tags, we allow both.
    let shape_id = (tag_ws("Shape_ID") | tag_ws("shape_id")) + int() + nl();
    let num_samples = (tag_ws("Num_Uncompressed") | tag_ws("num_samples")) + int() + nl();
    let samples = num_samples + (ws().opt() + float() + nl()).repeat(1..);
    shape_id + samples
}
