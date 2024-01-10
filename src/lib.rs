use sequence::Sequence;

pub mod errors;
pub mod parse_file;
pub mod sequence;

pub fn parse_file(source: &str) -> Result<sequence::Sequence, errors::ParseError> {
    parse_file::parse_file(source).and_then(Sequence::from_parsed_file)
}
