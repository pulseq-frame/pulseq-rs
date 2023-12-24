use ezpc::*;
use std::collections::HashMap;

use super::common::*;
use super::*;

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

fn parse_defs(defs: Vec<(String, String)>) -> Result<Definitions, ParseError> {
    let mut defs: HashMap<_, _> = defs.into_iter().collect();

    fn parse_fov(s: String) -> Result<(f32, f32, f32), ParseError> {
        let splits: Vec<_> = s.split_whitespace().collect();
        if splits.len() != 3 {
            return Err(ParseError::Generic);
        }
        Ok((splits[0].parse()?, splits[1].parse()?, splits[2].parse()?))
    }

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
        total_duration: defs
            .remove("TotalDuration")
            .map(|s| s.parse())
            .transpose()?,
        rest: defs,
    })
}

fn version() -> Parser<impl Parse<Output = Version>> {
    let major = tag_ws("major") + tag_nl("1");
    let minor = tag_ws("minor") + tag_nl("4");
    let revision = tag_ws("revision") + tag("0") + ident().opt() + nl();

    (tag_nl("[VERSION]") + major + minor + revision).map(|rev_suppl| Version {
        major: 1,
        minor: 4,
        revision: 0,
        rev_suppl,
    })
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
    let def = ident() + ws() + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + nl();
    tag_nl("[DEFINITIONS]") + def.repeat(1..).try_map(parse_defs)
}

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (ws().opt() + integer() + (ws() + integer()).repeat(7)).map(|(id, tags)| Block {
        id,
        duration: tags[0],
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
    let i = || ws() + integer();
    let f = || ws() + float();
    let rf = (ws().opt() + integer() + f() + i() + i() + i() + i() + f() + f()).map(
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
    let i = || ws() + integer();
    let f = ws() + float();
    let grad = (ws().opt() + integer() + f + i() + i() + i()).map(
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

fn traps() -> Parser<impl Parse<Output = Vec<Trap>>> {
    let i = || ws() + integer();
    let f = ws() + float();
    let trap = (ws().opt() + integer() + f + i() + i() + i() + i()).map(
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

fn adcs() -> Parser<impl Parse<Output = Vec<Adc>>> {
    let i = || ws() + integer();
    let f = || ws() + float();
    let adc = (ws().opt() + integer() + i() + f() + i() + f() + f()).map(
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

fn extensions() -> Parser<impl Parse<Output = Extensions>> {
    let rest_of_line = none_of("\n").repeat(1..).map(|s| s.trim().to_owned());
    let i = || ws() + integer();
    let ext_ref =
        (ws().opt() + integer() + i() + i() + i() + nl()).map(|(((id, spec_id), obj_id), next)| {
            ExtensionRef {
                id,
                spec_id,
                obj_id,
                next,
            }
        });
    let ext_obj = (ws().opt() + integer() + rest_of_line + nl())
        .map(|(id, data)| ExtensionObject { id, data });
    let ext_spec = (tag_ws("extension") + ident() + ws() + integer() + nl() + ext_obj.repeat(1..))
        .map(|((name, id), instances)| ExtensionSpec {
            id,
            name,
            instances,
        });
    (tag_nl("[EXTENSIONS]") + ext_ref.repeat(1..) + ext_spec.repeat(1..))
        .map(|(refs, specs)| Extensions { refs, specs })
}

fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    // The spec and the exporter use different tags, we allow both.
    let shape_id = (tag_ws("Shape_ID") | tag_ws("shape_id")) + integer() + nl();
    let num_samples = (tag_ws("Num_Uncompressed") | tag_ws("num_samples")) + integer() + nl();
    let samples = (num_samples + (ws().opt() + float() + nl()).repeat(1..))
        .try_map(|(num_samples, samples)| decompress_shape(samples, num_samples));

    let shape = (shape_id + samples).map(|(id, samples)| Shape { id, samples });
    tag_nl("[SHAPES]") + shape.repeat(1..)
}
