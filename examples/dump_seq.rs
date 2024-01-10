use pulseq_rs::Sequence;
use std::fs::File;
use std::io::Write;

fn main() {
    let seq = Sequence::from_file("assets/1.4.0/epi_se_rs.seq").unwrap();
    let mut out = File::create("assets/epi_se_rs.seq.dump").unwrap();
    write!(out, "{seq}").unwrap();
}
