use pulseq_rs::Sequence;

#[test]
fn epi_label() {
    assert!(Sequence::from_file("assets/1.4.0/epi_label.seq").is_ok());
}
#[test]
fn epi_se_rs() {
    assert!(Sequence::from_file("assets/1.4.0/epi_se_rs.seq").is_ok());
}
#[test]
fn epi_se() {
    assert!(Sequence::from_file("assets/1.4.0/epi_se.seq").is_ok());
}
#[test]
fn epi() {
    assert!(Sequence::from_file("assets/1.4.0/epi.seq").is_ok());
}
#[test]
fn gre_label() {
    assert!(Sequence::from_file("assets/1.4.0/gre_label.seq").is_ok());
}
#[test]
fn gre_radial() {
    assert!(Sequence::from_file("assets/1.4.0/gre_radial.seq").is_ok());
}
#[test]
fn gre() {
    assert!(Sequence::from_file("assets/1.4.0/gre.seq").is_ok());
}
#[test]
fn haste() {
    assert!(Sequence::from_file("assets/1.4.0/haste.seq").is_ok());
}
#[test]
fn mprage() {
    assert!(Sequence::from_file("assets/1.4.0/mprage.seq").is_ok());
}
#[test]
fn tse() {
    assert!(Sequence::from_file("assets/1.4.0/tse.seq").is_ok());
}
#[test]
fn ute() {
    assert!(Sequence::from_file("assets/1.4.0/ute.seq").is_ok());
}
