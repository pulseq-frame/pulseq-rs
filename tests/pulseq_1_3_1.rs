use pulseq_rs::Sequence;

#[test]
fn epi_se_rs() {
    assert!(Sequence::from_file("assets/1.3.1.post1/epi_se_rs.seq").is_ok());
}
#[test]
fn epi_se() {
    assert!(Sequence::from_file("assets/1.3.1.post1/epi_se.seq").is_ok());
}
#[test]
fn epi() {
    assert!(Sequence::from_file("assets/1.3.1.post1/epi.seq").is_ok());
}
#[test]
fn gre_label() {
    assert!(Sequence::from_file("assets/1.3.1.post1/gre_label.seq").is_ok());
}
#[test]
fn gre() {
    assert!(Sequence::from_file("assets/1.3.1.post1/gre.seq").is_ok());
}
#[test]
fn haste() {
    assert!(Sequence::from_file("assets/1.3.1.post1/haste.seq").is_ok());
}
#[test]
fn tse() {
    assert!(Sequence::from_file("assets/1.3.1.post1/tse.seq").is_ok());
}
#[test]
fn ute() {
    assert!(Sequence::from_file("assets/1.3.1.post1/ute.seq").is_ok());
}
