use crate::Tag;

/// A field value whose bytes can't be read as the field's declared data type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidValue {
    pub tag: Tag,
}

impl std::fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid value in field {}", self.tag)
    }
}

impl std::error::Error for InvalidValue {}

/// A value outside the range of the type it converts to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfRange;

impl std::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("value out of range")
    }
}

impl std::error::Error for OutOfRange {}
