use pulseq_rs::parse_file;

#[test]
fn epi_se_rs() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/epi_se_rs.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn epi_se() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/epi_se.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn epi() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/epi.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn gre_label() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/gre_label.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn gre() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/gre.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn haste() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/haste.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn tse() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/tse.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}
#[test]
fn ute() {
    let source = std::fs::read_to_string("assets/1.3.1.post1/ute.seq").unwrap();
    assert!(parse_file(&source).is_ok());
}