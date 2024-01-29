use pulseq_rs::Sequence;

#[test]
fn epi_label() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/epi_label.seq").unwrap();
}
#[test]
fn epi_se_rs() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/epi_se_rs.seq").unwrap();
}
#[test]
fn epi_se() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/epi_se.seq").unwrap();
}
#[test]
fn epi() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/epi.seq").unwrap();
}
#[test]
fn gre_label() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/gre_label.seq").unwrap();
}
#[test]
fn gre_radial() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/gre_radial.seq").unwrap();
}
#[test]
fn gre() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/gre.seq").unwrap();
}
#[test]
fn haste() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/haste.seq").unwrap();
}
#[test]
fn mprage() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/mprage.seq").unwrap();
}
#[test]
fn tse() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/tse.seq").unwrap();
}
#[test]
fn ute() {
    Sequence::from_file("../test-seqs/pypulseq/1.4.0/ute.seq").unwrap();
}
