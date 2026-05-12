#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
use pulseq_rs::seq::Sequence;

#[test]
fn epi() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/epi.seq").unwrap();
}
#[test]
fn epi_label() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/epi_label.seq").unwrap();
}
#[test]
fn epi_se() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/epi_se.seq").unwrap();
}
#[test]
fn epi_se_rs() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/epi_se_rs.seq").unwrap();
}
#[test]
fn gre() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/gre.seq").unwrap();
}
#[test]
fn gre_label() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/gre_label.seq").unwrap();
}
#[test]
fn gre_label_softdelay() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/gre_label_softdelay.seq").unwrap();
}
#[test]
fn gre_radial() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/gre_radial.seq").unwrap();
}
#[test]
fn haste() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/haste.seq").unwrap();
}
#[test]
fn mprage() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/mprage.seq").unwrap();
}
#[test]
fn tse() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/tse.seq").unwrap();
}
#[test]
fn ute() {
    Sequence::from_file("../test-seqs/pypulseq/1.5.0.post1/ute.seq").unwrap();
}
