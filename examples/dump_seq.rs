#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
)]

use pulseq_rs::seq::Sequence;
use std::fs::File;
use std::io::Write;

fn main() {
    let seq = Sequence::from_file("assets/flash_je.seq").unwrap();
    let mut out = File::create("assets/flash_je.seq.dump").unwrap();
    write!(out, "{seq}").unwrap();
}
