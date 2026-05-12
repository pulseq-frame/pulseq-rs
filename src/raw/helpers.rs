use winnow::ascii::{line_ending, till_line_ending};
use winnow::combinator::{alt, eof, opt, preceded, repeat};
use winnow::prelude::*;
use winnow::token::take_while;

use crate::error::ShapeDecompressionError;

pub fn decompress_shape(
    samples: Vec<f64>,
    num_samples: u32,
) -> Result<Vec<f64>, ShapeDecompressionError> {
    // First, decompress into the deriviate of the shape
    let mut deriv = Vec::with_capacity(num_samples as usize);

    // The two samples before the current one, to detect RLE
    let mut a = f64::NAN;
    let mut b = f64::NAN;
    // After a detected RLE, skip the RLE check for two samples
    let mut skip: u32 = 0;

    for (index, sample) in samples.into_iter().enumerate() {
        if a == b && skip == 0 {
            if sample != sample.round() {
                Err(ShapeDecompressionError::RleCountIsNotInteger {
                    index,
                    value: sample,
                })?;
            }

            skip = 2;
            for _ in 0..sample as usize {
                deriv.push(b);
            }
        } else {
            skip = skip.saturating_sub(1);
            deriv.push(sample);
        }

        a = b;
        b = sample;
    }

    if deriv.len() != num_samples as usize {
        Err(ShapeDecompressionError::WrongDecompressedCount {
            count: deriv.len(),
            expected: num_samples as usize,
        })?;
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

/// Linearly extrapolate the normalized boundary amplitudes of a gradient
/// shape for pulseq < 1.5 files, which do not store explicit `first`/`last`.
/// Returns `(first, last)` in normalized units (multiply by `amp` for Hz/m).
///
/// - `time_id == 0` (samples sit at uniform half-tick centers
///   `[0.5, 1.5, …, M-0.5]`): boundary values lie outside the sampled set,
///   so we linearly extrapolate:
///   - `first = 0.5 * (3*s[0]   - s[1])`
///   - `last  = 0.5 * (3*s[M-1] - s[M-2])`
/// - `time_id != 0` (samples include the boundaries): read directly.
///   - `first = s[0]`, `last = s[M-1]`
/// - `N == 1`: degenerate, return `(s[0], s[0])`.
pub fn extrapolate_grad_boundaries(samples: &[f64], time_id: i32) -> (f64, f64) {
    let first = match samples {
        [] => f64::NAN,
        [a] => *a,
        [a, b, ..] if time_id == 0 => 1.5 * a - 0.5 * b,
        [a, ..] => *a
    };
    let last = match samples {
        [] => f64::NAN,
        [a] => *a,
        [.., b, a] if time_id == 0  => 1.5 * a - 0.5 * b,
        [.., b] => *b
    };
    (first, last)
}

// Simple parsers that are not really specific to pulseq

/// Matches at least one whitespace but now newline
pub fn ws(input: &mut &str) -> ModalResult<()> {
    take_while(1.., (' ', '\t')).void().parse_next(input)
}

/// Matches as many whitespaces and comments as possible but expects at least one '\n'
pub fn nl(input: &mut &str) -> ModalResult<()> {
    // matches comments or empty lines, stops at line ending
    let comment = || alt((ws, ('#', till_line_ending).void()));
    // consume the line ending here not in comment to support ending in comment
    alt((
        eof.void(),
        (
            repeat::<_, _, (), _, _>(1.., (opt(comment()), line_ending)),
            opt(comment()),
        )
            .void(),
    ))
    .parse_next(input)
}

/// Shorthand for tag + whitespace
pub fn tag_ws(tag_str: &'static str) -> impl FnMut(&mut &str) -> ModalResult<()> {
    move |input: &mut &str| (tag_str, ws).void().parse_next(input)
}

/// Shorthand for tag + newline
pub fn tag_nl(tag_str: &'static str) -> impl FnMut(&mut &str) -> ModalResult<()> {
    move |input: &mut &str| (tag_str, nl).void().parse_next(input)
}

pub fn ident(input: &mut &str) -> ModalResult<String> {
    take_while(1.., |c: char| c.is_ascii_graphic())
        .map(str::to_owned)
        .parse_next(input)
}

/// (opt(ws), int)
pub fn int(input: &mut &str) -> ModalResult<u32> {
    preceded(opt(ws), winnow::ascii::dec_uint).parse_next(input)
}

/// (opt(ws), signed int) - used for `time_id` fields where pulseq 1.5 introduced
/// the `-1` sentinel (samples at every half-tick).
pub fn signed_int(input: &mut &str) -> ModalResult<i32> {
    preceded(opt(ws), winnow::ascii::dec_int).parse_next(input)
}

/// (opt(ws), float)
pub fn float(input: &mut &str) -> ModalResult<f64> {
    preceded(opt(ws), winnow::ascii::float).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::extrapolate_grad_boundaries;

    #[test]
    fn empty_samples_return_nan() {
        let (first, last) = extrapolate_grad_boundaries(&[], 0);
        assert!(first.is_nan() && last.is_nan());
    }

    #[test]
    fn single_sample_returns_that_sample() {
        for time_id in [0, -1, 7] {
            assert_eq!(
                extrapolate_grad_boundaries(&[0.42], time_id),
                (0.42, 0.42)
            );
        }
    }

    #[test]
    fn uniform_centers_linear_extrapolation() {
        // Linear ramp s[i] = i + 0.5 (i.e. samples at the half-tick centers
        // of a y = t line) should extrapolate to first = 0, last = N.
        let samples = [0.5, 1.5, 2.5, 3.5];
        let (first, last) = extrapolate_grad_boundaries(&samples, 0);
        assert!((first - 0.0).abs() < 1e-12);
        assert!((last - 4.0).abs() < 1e-12);
    }

    #[test]
    fn uniform_centers_two_samples() {
        // s = [0.5, 1.5]: first = 0.5*(3*0.5 - 1.5) = 0, last = 0.5*(3*1.5 - 0.5) = 2.
        let (first, last) = extrapolate_grad_boundaries(&[0.5, 1.5], 0);
        assert!((first - 0.0).abs() < 1e-12);
        assert!((last - 2.0).abs() < 1e-12);
    }

    #[test]
    fn extended_trapezoid_uses_endpoints_directly() {
        let samples = [0.0, 0.5, 1.0, 0.5, 0.0];
        assert_eq!(extrapolate_grad_boundaries(&samples, 1), (0.0, 0.0));
        assert_eq!(extrapolate_grad_boundaries(&samples, -1), (0.0, 0.0));
    }
}
