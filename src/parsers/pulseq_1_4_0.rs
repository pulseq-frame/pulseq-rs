use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

use ezpc::*;

#[cfg(test)]
mod tests {
    #[test]
    fn epi_label() {
        let source = std::fs::read_to_string("assets/1.4.0/epi_label.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn epi_se_rs() {
        let source = std::fs::read_to_string("assets/1.4.0/epi_se_rs.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn epi() {
        let source = std::fs::read_to_string("assets/1.4.0/epi.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn gre_label() {
        let source = std::fs::read_to_string("assets/1.4.0/gre_label.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn gre_radial() {
        let source = std::fs::read_to_string("assets/1.4.0/gre_radial.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn gre() {
        let source = std::fs::read_to_string("assets/1.4.0/gre.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn haste() {
        let source = std::fs::read_to_string("assets/1.4.0/haste.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn mprage() {
        let source = std::fs::read_to_string("assets/1.4.0/mprage.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn tse() {
        let source = std::fs::read_to_string("assets/1.4.0/tse.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
    #[test]
    fn ute() {
        let source = std::fs::read_to_string("assets/1.4.0/ute.seq").unwrap();
        assert!(super::file().parse_all(&source).is_ok());
    }
}

#[derive(Debug)]
enum Section {
    Version(Version),
    Signature(Signature),
    Definitions(Definitions),
    Blocks(Vec<Block>),
    Rfs(Vec<Rf>),
    Gradients(Vec<Gradient>),
    Traps(Vec<Trap>),
    Adcs(Vec<Adc>),
    Extensions(Extensions),
    Shapes(Vec<Shape>),
}

fn file() -> Parser<impl Parse<Output = Vec<Section>>> {
    newline().opt()
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

#[derive(Debug)]
struct Version {
    major: u32,
    minor: u32,
    revision: u32,
    rev_suppl: Option<String>,
}

#[derive(Debug)]
struct Signature {
    typ: String,
    hash: String,
}

#[derive(Debug)]
struct Definitions {
    grad_raster: f32,
    rf_raster: f32,
    adc_raster: f32,
    block_dur_raster: f32,
    name: Option<String>,
    fov: Option<(f32, f32, f32)>,
    total_duration: Option<f32>,
    rest: HashMap<String, String>,
}

#[derive(Debug)]
struct Block {
    id: u32,
    duration: u32,
    rf: u32,
    gx: u32,
    gy: u32,
    gz: u32,
    adc: u32,
    ext: u32,
}

#[derive(Debug)]
struct Rf {
    id: u32,
    /// `Hz`
    amp: f32,
    mag_id: u32,
    phase_id: u32,
    time_id: u32,
    /// `s` (from pulseq: `us`)
    delay: f32,
    /// `Hz`
    freq: f32,
    /// `rad`
    phase: f32,
}

#[derive(Debug)]
struct Gradient {
    id: u32,
    /// `Hz/m`
    amp: f32,
    shape_id: u32,
    time_id: u32,
    /// `s` (from pulseq: `us`)
    delay: f32,
}

#[derive(Debug)]
struct Trap {
    id: u32,
    /// `Hz/m`
    amp: f32,
    /// `s` (from pulseq: `us`)
    rise: f32,
    /// `s` (from pulseq: `us`)
    flat: f32,
    /// `s` (from pulseq: `us`)
    fall: f32,
    /// `s` (from pulseq: `us`)
    delay: f32,
}

#[derive(Debug)]
struct Adc {
    id: u32,
    num: u32,
    /// `s` (from pulseq: `ns`)
    dwell: f32,
    /// `s` (from pulseq: `us`)
    delay: f32,
    /// `Hz`
    freq: f32,
    /// `rad`
    phase: f32,
}

#[derive(Debug)]
struct ExtensionSpec {
    id: u32,
    name: String,
    instances: Vec<ExtensionObject>,
}

#[derive(Debug)]
struct ExtensionObject {
    id: u32,
    data: String,
}

#[derive(Debug)]
struct ExtensionRef {
    id: u32,
    spec_id: u32,
    obj_id: u32,
    next: u32,
}

#[derive(Debug)]
struct Extensions {
    refs: Vec<ExtensionRef>,
    specs: Vec<ExtensionSpec>,
}

#[derive(Debug)]
struct Shape {
    id: u32,
    samples: Vec<f32>,
}

impl Definitions {
    fn parse(defs: Vec<(String, String)>) -> Result<Self, ParseError> {
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
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("GENERIC ERROR - IMPLEMENT MORE DETAILED VARIANTS")]
    Generic,
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

fn whitespace() -> Matcher<impl Match> {
    one_of(" \t").repeat(1..)
}

/// Matches as many whitespaces and comments as possible but expects at least one '\n'
fn newline() -> Matcher<impl Match> {
    let ignore = || whitespace() | (tag("#") + none_of("\n").repeat(0..));
    let eol = || tag("\r\n") | tag("\n");

    eof() | ((ignore().opt() + eol()).repeat(1..) + ignore().opt())
}

fn ident() -> Parser<impl Parse<Output = String>> {
    is_a(char::is_alphanumeric)
        .repeat(1..)
        .map(|s| s.to_owned())
}

fn integer() -> Parser<impl Parse<Output = u32>> {
    (tag("0") | (one_of("123456789") + one_of("0123456789").repeat(0..))).try_map(|s| s.parse())
}

fn float() -> Parser<impl Parse<Output = f32>> {
    let integer = tag("0") | (one_of("123456789") + one_of("0123456789").repeat(0..));
    let frac = tag(".") + one_of("0123456789").repeat(1..);
    let exp = one_of("eE") + one_of("+-").opt() + one_of("0123456789").repeat(1..);
    let number = tag("-").opt() + integer + frac.opt() + exp.opt();
    number.try_map(|s| f32::from_str(s))
}

fn version() -> Parser<impl Parse<Output = Version>> {
    let major = tag("major") + whitespace() + tag("1") + newline();
    let minor = tag("minor") + whitespace() + tag("4") + newline();
    let revision = tag("revision") + whitespace() + tag("0") + ident().opt() + newline();

    (tag("[VERSION]") + newline() + major + minor + revision).map(|rev_suppl| Version {
        major: 1,
        minor: 4,
        revision: 0,
        rev_suppl,
    })
}

fn signature() -> Parser<impl Parse<Output = Signature>> {
    let typ = tag("Type")
        + whitespace()
        + is_a(char::is_alphanumeric)
            .repeat(1..)
            .map(|s| s.to_owned())
        + newline();
    let hash = tag("Hash")
        + whitespace()
        + none_of("\n").repeat(1..).map(|s| s.trim().to_owned())
        + newline();

    (tag("[SIGNATURE]") + newline() + typ + hash).map(|(typ, hash)| Signature { typ, hash })
}

fn definitions() -> Parser<impl Parse<Output = Definitions>> {
    let def =
        ident() + whitespace() + none_of("\n").repeat(1..).map(|s| s.trim().to_owned()) + newline();
    tag("[DEFINITIONS]") + newline() + def.repeat(1..).try_map(Definitions::parse)
}

fn blocks() -> Parser<impl Parse<Output = Vec<Block>>> {
    let block = (whitespace().opt() + integer() + (whitespace() + integer()).repeat(7)).map(
        |(id, tags)| Block {
            id,
            duration: tags[0],
            rf: tags[1],
            gx: tags[2],
            gy: tags[3],
            gz: tags[4],
            adc: tags[5],
            ext: tags[6],
        },
    );
    tag("[BLOCKS]") + newline() + (block + newline()).repeat(1..)
}

fn rfs() -> Parser<impl Parse<Output = Vec<Rf>>> {
    let i = || whitespace() + integer();
    let f = || whitespace() + float();
    let rf = (whitespace().opt() + integer() + f() + i() + i() + i() + i() + f() + f()).map(
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
    tag("[RF]") + newline() + (rf + newline()).repeat(1..)
}

fn gradients() -> Parser<impl Parse<Output = Vec<Gradient>>> {
    let i = || whitespace() + integer();
    let f = whitespace() + float();
    let grad = (whitespace().opt() + integer() + f + i() + i() + i()).map(
        |((((id, amp), shape_id), time_id), delay)| Gradient {
            id,
            amp,
            shape_id,
            time_id,
            delay: delay as f32 * 1e-6,
        },
    );
    tag("[GRADIENTS]") + newline() + (grad + newline()).repeat(1..)
}

fn traps() -> Parser<impl Parse<Output = Vec<Trap>>> {
    let i = || whitespace() + integer();
    let f = whitespace() + float();
    let trap = (whitespace().opt() + integer() + f + i() + i() + i() + i()).map(
        |(((((id, amp), rise), flat), fall), delay)| Trap {
            id,
            amp,
            rise: rise as f32 * 1e-6,
            flat: flat as f32 * 1e-6,
            fall: fall as f32 * 1e-6,
            delay: delay as f32 * 1e-6,
        },
    );
    tag("[TRAP]") + newline() + (trap + newline()).repeat(1..)
}

fn adcs() -> Parser<impl Parse<Output = Vec<Adc>>> {
    let i = || whitespace() + integer();
    let f = || whitespace() + float();
    let adc = (whitespace().opt() + integer() + i() + f() + i() + f() + f()).map(
        |(((((id, num), dwell), delay), freq), phase)| Adc {
            id,
            num,
            dwell: dwell * 1e-9,
            delay: delay as f32 * 1e-6,
            freq,
            phase,
        },
    );
    tag("[ADC]") + newline() + (adc + newline()).repeat(1..)
}

fn extensions() -> Parser<impl Parse<Output = Extensions>> {
    let rest_of_line = none_of("\n").repeat(1..).map(|s| s.trim().to_owned());
    let i = || whitespace() + integer();
    let ext_ref = (whitespace().opt() + integer() + i() + i() + i() + newline()).map(
        |(((id, spec_id), obj_id), next)| ExtensionRef {
            id,
            spec_id,
            obj_id,
            next,
        },
    );
    let ext_obj = (whitespace().opt() + integer() + rest_of_line + newline())
        .map(|(id, data)| ExtensionObject { id, data });
    let ext_spec = (tag("extension")
        + whitespace()
        + ident()
        + whitespace()
        + integer()
        + newline()
        + ext_obj.repeat(1..))
    .map(|((name, id), instances)| ExtensionSpec {
        id,
        name,
        instances,
    });
    (tag("[EXTENSIONS]") + newline() + ext_ref.repeat(1..) + ext_spec.repeat(1..))
        .map(|(refs, specs)| Extensions { refs, specs })
}

fn shapes() -> Parser<impl Parse<Output = Vec<Shape>>> {
    // The spec and the exporter use different tags, we allow both.
    let shape_id = (tag("Shape_ID") | tag("shape_id")) + whitespace() + integer() + newline();
    let num_samples =
        (tag("Num_Uncompressed") | tag("num_samples")) + whitespace() + integer() + newline();
    let samples = (num_samples + (whitespace().opt() + float() + newline()).repeat(1..))
        .try_map(|(num_samples, samples)| decompress_shape(samples, num_samples));

    let shape = (shape_id + samples).map(|(id, samples)| Shape { id, samples });
    tag("[SHAPES]") + newline() + shape.repeat(1..)
}

fn decompress_shape(samples: Vec<f32>, num_samples: u32) -> Result<Vec<f32>, ParseError> {
    if samples.len() as u32 == num_samples {
        return Ok(samples);
    }

    // First, decompress into the deriviate of the shape
    let mut deriv = Vec::with_capacity(num_samples as usize);

    let mut a = f32::NAN;
    let mut b = f32::NAN;
    for sample in samples {
        if a == b {
            if sample != sample.round() {
                return Err(ParseError::Generic);
            }
            for _ in 0..sample as usize {
                deriv.push(b);
            }
        } else {
            deriv.push(sample);
        }

        a = b;
        b = sample;
    }

    if deriv.len() != num_samples as usize {
        return Err(ParseError::Generic);
    }

    // Then, do a cumultative sum to get the shape
    Ok(deriv
        .into_iter()
        .scan(0.0, |acc, x| {
            *acc += x;
            Some(*acc)
        })
        .collect())
}
