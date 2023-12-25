use crate::parsers::pulseq_1_4::file;

#[test]
fn epi_label() {
    let source = std::fs::read_to_string("assets/1.4.0/epi_label.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn epi_se_rs() {
    let source = std::fs::read_to_string("assets/1.4.0/epi_se_rs.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn epi_se() {
    let source = std::fs::read_to_string("assets/1.4.0/epi_se.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn epi() {
    let source = std::fs::read_to_string("assets/1.4.0/epi.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn gre_label() {
    let source = std::fs::read_to_string("assets/1.4.0/gre_label.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn gre_radial() {
    let source = std::fs::read_to_string("assets/1.4.0/gre_radial.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn gre() {
    let source = std::fs::read_to_string("assets/1.4.0/gre.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn haste() {
    let source = std::fs::read_to_string("assets/1.4.0/haste.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn mprage() {
    let source = std::fs::read_to_string("assets/1.4.0/mprage.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn tse() {
    let source = std::fs::read_to_string("assets/1.4.0/tse.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
#[test]
fn ute() {
    let source = std::fs::read_to_string("assets/1.4.0/ute.seq").unwrap();
    assert!(file().parse_all(&source).is_ok());
}
