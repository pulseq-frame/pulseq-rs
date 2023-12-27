pub mod parsers;
pub mod sequence;
#[cfg(test)]
mod tests;

pub fn parse_file(source: &str) -> Result<sequence::Sequence, parsers::common::ParseError> {
    parsers::parse_file(source).map(sequence::Sequence::from_1_4)
}
