use pulseq_rs::Sequence;

#[test]
fn epi() {
    Sequence::from_file("../test-seqs/pypulseq/1.2.0.post4/epi_pypulseq.seq").unwrap();
}
#[test]
fn epi_rs() {
    Sequence::from_file("../test-seqs/pypulseq/1.2.0.post4/epi_rs_pypulseq.seq").unwrap();
}
#[test]
fn gre() {
    Sequence::from_file("../test-seqs/pypulseq/1.2.0.post4/gre_pypulseq.seq").unwrap();
}
#[test]
fn haste() {
    Sequence::from_file("../test-seqs/pypulseq/1.2.0.post4/haste_pypulseq.seq").unwrap();
}
#[test]
fn tse() {
    Sequence::from_file("../test-seqs/pypulseq/1.2.0.post4/tse_pypulseq.seq").unwrap();
}
