#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
use pulseq_rs::seq::Sequence;

#[test]
fn radial() {
    Sequence::from_file("assets/seq_make_radial.seq").unwrap();
}
