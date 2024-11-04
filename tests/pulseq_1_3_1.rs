use pulseq_rs::Sequence;

#[test]
fn epi_se_rs() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/epi_se_rs.seq").unwrap();
}
#[test]
fn epi_se() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/epi_se.seq").unwrap();
}
#[test]
fn epi() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/epi.seq").unwrap();
}
#[test]
fn gre_label() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/gre_label.seq").unwrap();
}
#[test]
fn gre() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/gre.seq").unwrap();
}
#[test]
fn haste() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/haste.seq").unwrap();
}
#[test]
fn tse() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/tse.seq").unwrap();
}
#[test]
fn ute() {
    Sequence::from_file("../test-seqs/pypulseq/1.3.1.post1/ute.seq").unwrap();
}
#[test]
fn rfshim() {
    Sequence::from_file("../test-seqs/pypulseq_rf_shim/B1map_presat_4adc_pythonby_rfshim.seq")
        .unwrap();
}
