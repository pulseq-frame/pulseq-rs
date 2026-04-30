use pulseq_rs::seq::Sequence;

#[test]
fn radial() {
    Sequence::from_file("assets/seq_make_radial.seq").unwrap();
}
