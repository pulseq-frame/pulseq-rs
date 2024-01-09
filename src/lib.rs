pub mod parsers;
pub mod sequence;

pub fn parse_file(source: &str) -> Result<sequence::Sequence, parsers::helpers::ParseError> {
    parsers::parse_file(source).and_then(sequence::Sequence::from_1_4)
}
