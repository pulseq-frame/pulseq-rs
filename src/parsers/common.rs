use ezpc::*;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("GENERIC ERROR - IMPLEMENT MORE DETAILED VARIANTS")]
    Generic,
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

pub fn ws() -> Matcher<impl Match> {
    one_of(" \t").repeat(1..)
}

/// Matches as many whitespaces and comments as possible but expects at least one '\n'
pub fn nl() -> Matcher<impl Match> {
    let ignore = || ws() | (tag("#") + none_of("\n").repeat(0..));
    let eol = || tag("\r\n") | tag("\n");

    eof() | ((ignore().opt() + eol()).repeat(1..) + ignore().opt())
}

/// Shorthand for tag + whitespace
pub fn tag_ws(tag_str: &'static str) -> Matcher<impl Match> {
    tag(tag_str) + ws()
}

/// Shorthand for tag + newline
pub fn tag_nl(tag_str: &'static str) -> Matcher<impl Match> {
    tag(tag_str) + nl()
}

pub fn ident() -> Parser<impl Parse<Output = String>> {
    is_a(char::is_alphanumeric)
        .repeat(1..)
        .map(|s| s.to_owned())
}

pub fn integer() -> Parser<impl Parse<Output = u32>> {
    (tag("0") | (one_of("123456789") + one_of("0123456789").repeat(0..))).try_map(|s| s.parse())
}

pub fn float() -> Parser<impl Parse<Output = f32>> {
    let integer = tag("0") | (one_of("123456789") + one_of("0123456789").repeat(0..));
    let frac = tag(".") + one_of("0123456789").repeat(1..);
    let exp = one_of("eE") + one_of("+-").opt() + one_of("0123456789").repeat(1..);
    let number = tag("-").opt() + integer + frac.opt() + exp.opt();
    number.try_map(|s| f32::from_str(s))
}

pub fn decompress_shape(samples: Vec<f32>, num_samples: u32) -> Result<Vec<f32>, ParseError> {
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
