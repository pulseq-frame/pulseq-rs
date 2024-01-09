use pulseq_rs::parse_file;
use std::fs::{read_to_string, File};
use std::io::Write;

fn main() {
    let source = read_to_string("assets/1.4.0/epi_se_rs.seq").unwrap();
    let seq = parse_file(&source).unwrap();
    let mut out = File::create("assets/epi_se_rs.seq.dump").unwrap();
    write!(out, "{seq}").unwrap();
}
